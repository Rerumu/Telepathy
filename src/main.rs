use std::{
	fs::File,
	io::{BufWriter, Write},
};

use argh::FromArgs;
use regioned::{
	data_flow::link::Id,
	dot::Dot,
	transform::{
		relax_dependencies::RelaxDependencies,
		retain_only,
		revise::{self, redo_ports, redo_ports_in_place},
	},
	visit::{reverse_topological::ReverseTopological, successors::Successors},
};
use telepathy::{
	codegen,
	hir::{
		data::{Node, Nodes, Simple},
		isle::{self, Elided},
		parser::{ParseData, Parser},
	},
	mir::{data::Program, sequencer::Sequencer},
};

/// A `BrainFxck` optimizing compiler based on the `Regioned` implementation of
/// a Regionalized Value State Dependence Graph.
#[allow(clippy::struct_excessive_bools)]
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
	constant_fold: bool,

	/// whether load and store elision should be performed
	#[argh(switch)]
	load_store_elide: bool,

	/// whether to relax dependencies of compounds
	#[argh(switch)]
	relax_dependencies: bool,
}

fn run_fold_identity(successors: &Successors) -> impl FnMut(&mut Nodes, Id) -> Option<Node> + '_ {
	revise::single(
		|nodes, id| isle::identity(nodes, id.into()),
		|nodes, id, value| {
			redo_ports(nodes, successors, id, |port| (port == 0).then_some(value));

			Simple::NoOp.into()
		},
	)
}

fn run_fold_expressions() -> impl FnMut(&mut Nodes, Id) -> Option<Node> {
	revise::single(
		|nodes, id| isle::fold(nodes, id.into()),
		|_, _, math| Simple::from(math).into(),
	)
}

fn run_load_store_elision(
	successors: &Successors,
) -> impl FnMut(&mut Nodes, Id) -> Option<Node> + '_ {
	revise::single(
		|nodes, id| isle::elide(nodes, id.into()),
		|nodes, id, elided| {
			let result = match elided {
				Elided::Merge { state } => {
					redo_ports_in_place(nodes, successors, id, state.node);

					Simple::NoOp
				}
				Elided::Load { store, value } => {
					redo_ports(nodes, successors, id, |port| match port {
						0 => Some(store),
						1 => Some(value),
						_ => None,
					});

					Simple::NoOp
				}
				Elided::Store {
					store,
					pointer,
					value,
				} => Simple::Store {
					state: store,
					pointer,
					value,
				},
			};

			result.into()
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
	nodes: &mut Nodes,
	id: Id,
	arguments: &Arguments,
	successors: &Successors,
	relax: &mut RelaxDependencies,
) -> usize {
	let mut applied = 0;

	if arguments.constant_fold {
		if run_fold_identity(successors)(nodes, id).is_some() {
			applied += 1;
		}

		if run_fold_expressions()(nodes, id).is_some() {
			applied += 1;
		}
	}

	if arguments.load_store_elide && run_load_store_elision(successors)(nodes, id).is_some() {
		applied += 1;
	}

	if arguments.relax_dependencies
		&& applied == 0
		&& relax.run(nodes, id, successors).unwrap_or_default() != 0
	{
		applied += 1;
	}

	applied
}

fn process_hir(code: &str, arguments: &Arguments) -> ParseData {
	let mut data = Parser::new().parse(code.char_indices()).unwrap();

	let roots = data.roots();
	let mut list = Vec::new();
	let mut relax = RelaxDependencies::new();
	let mut successors = Successors::new();
	let mut topological = ReverseTopological::new();

	loop {
		list.clear();
		list.extend(topological.iter(data.nodes(), roots));

		successors.run(data.nodes(), roots, &mut topological);

		let applied = list.iter().fold(0, |acc, &id| {
			acc + run_optimization(data.nodes_mut(), id, arguments, &successors, &mut relax)
		});

		if applied == 0 {
			break;
		}
	}

	retain_only::run(data.nodes_mut(), roots, &mut topological);

	data
}

fn process_mir(data: &ParseData) -> Program {
	let mut topological = ReverseTopological::new();
	let mut sequencer = Sequencer::new();

	sequencer.sequence(data, &mut topological)
}

fn main() {
	let mut arguments = argh::from_env::<Arguments>();

	if arguments.optimize {
		arguments.constant_fold = true;
		arguments.load_store_elide = true;
		arguments.relax_dependencies = true;
	}

	let input = load_input(arguments.input.as_deref());
	let data = process_hir(&input, &arguments);

	let output = &mut load_output(arguments.output.as_deref());

	let result = match arguments.target.as_str() {
		"dot" => Dot::new().write(output, data.nodes(), data.roots()),
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
