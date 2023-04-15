use std::{
	fs::File,
	io::{BufWriter, Write},
};

use argh::FromArgs;
use regioned::{
	data_flow::{link::Port, node::Id},
	dot::Dot,
	transform::{
		revise::{self, redo_ports, redo_ports_in_place},
		sweep,
	},
	visit::{reverse_topological::ReverseTopological, successors::Successors},
};
use telepathy::{
	codegen,
	hir::{
		data::{Graph, Node, Simple},
		isle::{self, Elided, Math},
		parser::{ParseData, Parser},
	},
	mir::{data::Program, sequencer::Sequencer},
};

/// A `BrainFuck` optimizing compiler based on the `Regioned` implementation of
/// a Regionalized Value State Dependence Graph.
#[derive(FromArgs)]
struct Arguments {
	/// the target language to compile to,
	/// currently supported: `dot`, `c`, `lua`
	#[argh(positional)]
	target: String,

	/// the input file to compile
	/// if not specified, stdin is used
	#[argh(option, short = 'i')]
	input: Option<String>,

	/// the output file to write to
	/// if not specified, stdout is used
	#[argh(option, short = 'o')]
	output: Option<String>,

	/// whether all optimizations should be performed
	#[argh(switch, short = 'O')]
	optimize: bool,

	/// whether constant folding should be performed
	#[argh(switch)]
	constant_folding: bool,

	/// whether load and store elision should be performed
	#[argh(switch)]
	load_store_elision: bool,
}

fn run_fold_identity(successors: &Successors) -> impl FnMut(&mut Graph, Id) -> Option<Node> + '_ {
	revise::single(
		|graph, id| isle::identity(graph, id.into()),
		|graph, id, value| {
			let predecessors = &mut graph.predecessors;
			let redo_value = |port: Port| (port.index() == 0).then_some(value.port());

			redo_ports(predecessors, successors, id, value.node(), redo_value);

			Simple::NoOp.into()
		},
	)
}

fn run_fold_expressions() -> impl FnMut(&mut Graph, Id) -> Option<Node> {
	revise::single(
		|graph, id| isle::fold(graph, id.into()),
		|graph, id, math| {
			let predecessors = &mut graph.predecessors[id];

			predecessors.clear();

			match math {
				Math::Integer { value } => Simple::Integer(value).into(),
				Math::Add { lhs, rhs } => {
					predecessors.extend([lhs, rhs]);

					Simple::Add.into()
				}
				Math::Sub { lhs, rhs } => {
					predecessors.extend([lhs, rhs]);

					Simple::Sub.into()
				}
			}
		},
	)
}

fn run_load_store_elision(
	successors: &Successors,
) -> impl FnMut(&mut Graph, Id) -> Option<Node> + '_ {
	revise::single(
		|graph, id| isle::elide(graph, id.into()),
		|graph, id, elided| {
			let predecessors = &mut graph.predecessors;

			match elided {
				Elided::Merge { state } => {
					redo_ports_in_place(predecessors, successors, id, state.node());

					Simple::NoOp.into()
				}
				Elided::Load { store, value } => {
					let redo_store = |port: Port| (port.index() == 0).then_some(store.port());
					let redo_value = |port: Port| (port.index() == 1).then_some(value.port());

					redo_ports(predecessors, successors, id, store.node(), redo_store);
					redo_ports(predecessors, successors, id, value.node(), redo_value);

					Simple::NoOp.into()
				}
				Elided::Store { store } => {
					predecessors[id][0] = store;

					Simple::Store.into()
				}
			}
		},
	)
}

fn load_input(name: Option<&str>) -> String {
	if let Some(name) = name {
		std::fs::read_to_string(name).expect("failed to read input file")
	} else {
		let stdin = std::io::stdin().lock();

		std::io::read_to_string(stdin).expect("failed to read stdin")
	}
}

fn load_output(name: Option<&str>) -> Box<dyn Write> {
	if let Some(name) = name {
		let file = File::create(name).expect("failed to open output file");

		Box::new(BufWriter::new(file))
	} else {
		Box::new(std::io::stdout().lock())
	}
}

fn run_optimization(
	graph: &mut Graph,
	id: Id,
	arguments: &Arguments,
	successors: &Successors,
) -> usize {
	let mut applied = 0;

	if arguments.constant_folding {
		if run_fold_identity(successors)(graph, id).is_some() {
			applied += 1;
		}

		if run_fold_expressions()(graph, id).is_some() {
			applied += 1;
		}
	}

	if arguments.load_store_elision && run_load_store_elision(successors)(graph, id).is_some() {
		applied += 1;
	}

	applied
}

fn process_hir(code: &str, arguments: &Arguments) -> ParseData {
	let mut parser = Parser::new();
	let mut data = parser.parse(code.char_indices()).unwrap();

	let roots = data.roots();
	let mut successors = Successors::new();
	let mut topological = ReverseTopological::new();

	loop {
		let mut applied = 0;

		sweep::run(data.graph_mut(), roots, &mut topological);

		successors.run(data.graph(), roots, &mut topological);

		topological.run_with_mut(data.graph_mut(), roots, |graph, id| {
			applied += run_optimization(graph, id, arguments, &successors);
		});

		if applied == 0 {
			break;
		}
	}

	data
}

fn process_mir(data: &ParseData) -> Program {
	let mut topological = ReverseTopological::new();
	let mut sequencer = Sequencer::new();

	sequencer.sequence(data.graph(), data.roots(), &mut topological)
}

fn main() {
	let mut arguments = argh::from_env::<Arguments>();

	if arguments.optimize {
		arguments.constant_folding = true;
		arguments.load_store_elision = true;
	}

	let input = load_input(arguments.input.as_deref());
	let data = process_hir(&input, &arguments);

	let output = &mut load_output(arguments.output.as_deref());

	let result = match arguments.target.as_str() {
		"dot" => Dot::new(data.graph()).write(output, data.roots()),
		"c" => {
			let program = process_mir(&data);

			codegen::c89::write(output, &program)
		}
		"lua" => {
			let program = process_mir(&data);

			codegen::lua51::write(output, &program)
		}
		target => panic!("unsupported target `{target}`"),
	};

	result.expect("failed to write output");
}
