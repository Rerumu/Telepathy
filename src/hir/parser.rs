use std::str::CharIndices;

use regioned::data_flow::{
	link::Link,
	node::{Compound, Id, Region},
};

use super::data::{Builder, Graph, Simple};

pub struct ParseData {
	graph: Graph,
	io: Id,
}

impl ParseData {
	#[must_use]
	pub const fn graph(&self) -> &Graph {
		&self.graph
	}

	#[must_use]
	pub fn graph_mut(&mut self) -> &mut Graph {
		&mut self.graph
	}

	#[must_use]
	pub const fn roots(&self) -> [Id; 1] {
		[self.io]
	}
}

#[derive(Debug)]
pub enum ParseError {
	TooManyClosingBrackets { start: usize },
	TooLittleClosingBrackets,
}

#[derive(Default, Clone, Copy)]
struct Block {
	output: Id,
	parent: Id,
}

#[derive(Default)]
pub struct Parser {
	graph: Graph,
	blocks: Vec<Block>,

	load_states: Vec<Link>,
	store_state: Link,
	io_state: Link,
	pointer: Link,
}

impl Parser {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	fn add_load_direct(&mut self) -> Link {
		let params = [self.store_state, self.pointer];
		let mut load = self.graph.add_parametrized(Simple::Load, params).iter();

		self.load_states.push(load.next().unwrap());

		load.next().unwrap()
	}

	fn reconcile_store_state(&mut self) -> Link {
		match self.load_states.len() {
			0 => self.store_state,
			1 => self.load_states.pop().unwrap(),
			_ => {
				let states = self.load_states.drain(..);

				self.graph.add_parametrized(Simple::Merge, states)
			}
		}
	}

	fn add_store_direct(&mut self, value: Link) {
		let params = [self.reconcile_store_state(), self.pointer, value];
		let mut store = self.graph.add_parametrized(Simple::Store, params).iter();

		self.store_state = store.next().unwrap();
	}

	fn add_pointer_shift(&mut self, op: Simple) {
		let one = self.graph.add_integer(1);

		self.pointer = self.graph.add_parametrized(op, [self.pointer, one]);
	}

	fn add_memory_arithmetic(&mut self, op: Simple) {
		let temporary = self.add_load_direct();
		let one = self.graph.add_integer(1);
		let result = self.graph.add_parametrized(op, [temporary, one]);

		self.add_store_direct(result);
	}

	fn add_tell_output(&mut self) {
		let temporary = self.add_load_direct();
		let params = [self.io_state, temporary];

		self.io_state = self.graph.add_parametrized(Simple::Tell, params);
	}

	fn add_ask_input(&mut self) {
		let params = [self.io_state];
		let mut ask = self.graph.add_parametrized(Simple::Ask, params).iter();

		self.io_state = ask.next().unwrap();
		self.add_store_direct(ask.next().unwrap());
	}

	fn add_theta_handle(&mut self, parent: Region) -> Id {
		let (theta, region) = self.graph.add_compound(Compound::Theta);

		self.graph.add_passthrough(parent.start(), theta, 3);
		self.graph.add_passthrough(theta, parent.end(), 3);

		let mut inner = Link::from(region.start()).iter();

		self.io_state = inner.next().unwrap();
		self.store_state = inner.next().unwrap();
		self.pointer = inner.next().unwrap();

		region.end()
	}

	fn add_block_start(&mut self) {
		let on_false = self.graph.add_region();
		let on_true = self.graph.add_region();

		let gamma = self.graph.add_gamma([on_false, on_true].into());
		let condition = self.add_load_direct();
		let store_state = self.reconcile_store_state();

		self.graph.predecessors[gamma].extend([
			self.io_state,
			store_state,
			self.pointer,
			condition,
		]);

		self.graph.add_identity_handle(on_false);

		let output = self.add_theta_handle(on_true);

		self.blocks.push(Block {
			output,
			parent: gamma,
		});
	}

	fn add_block_end(&mut self, start: usize) -> Result<(), ParseError> {
		let block = self
			.blocks
			.pop()
			.ok_or(ParseError::TooManyClosingBrackets { start })?;

		let condition = self.add_load_direct();
		let store_state = self.reconcile_store_state();

		self.graph.predecessors[block.output].extend([
			self.io_state,
			store_state,
			self.pointer,
			condition,
		]);

		let mut iter = Link::from(block.parent).iter();

		self.io_state = iter.next().unwrap();
		self.store_state = iter.next().unwrap();
		self.pointer = iter.next().unwrap();

		Ok(())
	}

	fn initialize_graph(&mut self) {
		// Graph may be in an invalid state if parsing fails.
		let mut graph = Graph::new();

		self.io_state = graph.add_single(Simple::IO);
		self.store_state = graph.add_single(Simple::Memory);
		self.pointer = graph.add_integer(0);

		self.graph = graph;
		self.blocks.clear();
	}

	/// # Errors
	///
	/// Returns `ParseError::TooManyClosingBrackets` if there are more closing brackets than opening brackets.
	/// Returns `ParseError::TooLittleClosingBrackets` if there are more opening brackets than closing brackets.
	pub fn parse(&mut self, source: CharIndices) -> Result<ParseData, ParseError> {
		self.initialize_graph();

		for (i, c) in source {
			match c {
				'>' => self.add_pointer_shift(Simple::Add),
				'<' => self.add_pointer_shift(Simple::Sub),
				'+' => self.add_memory_arithmetic(Simple::Add),
				'-' => self.add_memory_arithmetic(Simple::Sub),
				'.' => self.add_tell_output(),
				',' => self.add_ask_input(),
				'[' => self.add_block_start(),
				']' => self.add_block_end(i)?,
				_ => {}
			}
		}

		if !self.blocks.is_empty() {
			return Err(ParseError::TooLittleClosingBrackets);
		}

		let graph = std::mem::take(&mut self.graph);
		let io = self.io_state.node();

		Ok(ParseData { graph, io })
	}
}
