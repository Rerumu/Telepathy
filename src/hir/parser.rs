use std::str::CharIndices;

use regioned::data_flow::{
	link::{Id, Link, Region},
	node::AsParametersMut,
};

use super::data::{Builder, Nodes, Simple};

pub struct ParseData {
	nodes: Nodes,
	io: Id,
}

impl ParseData {
	#[must_use]
	pub const fn nodes(&self) -> &Nodes {
		&self.nodes
	}

	#[must_use]
	pub fn nodes_mut(&mut self) -> &mut Nodes {
		&mut self.nodes
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
	nodes: Nodes,
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
		let load = self.nodes.add_simple(Simple::Load {
			state: self.store_state,
			pointer: self.pointer,
		});

		let mut iter = Link::from(load).iter();

		self.load_states.push(iter.next().unwrap());

		iter.next().unwrap()
	}

	fn reconcile_store_state(&mut self) -> Link {
		match self.load_states.len() {
			0 => self.store_state,
			1 => self.load_states.pop().unwrap(),
			_ => {
				let states = self.load_states.drain(..).collect();

				self.nodes.add_simple(Simple::Merge { states }).into()
			}
		}
	}

	fn add_store_direct(&mut self, value: Link) {
		let state = self.reconcile_store_state();
		let pointer = self.pointer;
		let store = self.nodes.add_simple(Simple::Store {
			state,
			pointer,
			value,
		});

		self.store_state = store.into();
	}

	fn add_pointer_shift<F>(&mut self, function: F)
	where
		F: FnOnce(Link, Link) -> Simple,
	{
		let one = self.nodes.add_integer(1);
		let result = self.nodes.add_simple(function(self.pointer, one));

		self.pointer = result.into();
	}

	fn add_memory_arithmetic<F>(&mut self, function: F)
	where
		F: FnOnce(Link, Link) -> Simple,
	{
		let temporary = self.add_load_direct();
		let one = self.nodes.add_integer(1);
		let result = self.nodes.add_simple(function(temporary, one));

		self.add_store_direct(result.into());
	}

	fn add_tell_output(&mut self) {
		let value = self.add_load_direct();
		let result = self.nodes.add_simple(Simple::Tell {
			state: self.io_state,
			value,
		});

		self.io_state = result.into();
	}

	fn add_ask_input(&mut self) {
		let ask = self.nodes.add_simple(Simple::Ask {
			state: self.io_state,
		});
		let mut iter = Link::from(ask).iter();

		self.io_state = iter.next().unwrap();
		self.add_store_direct(iter.next().unwrap());
	}

	fn add_theta_handle(&mut self, parent: Region) -> Id {
		let (theta, region) = self.nodes.add_theta();

		self.nodes.add_passthrough(parent.start(), theta, 3);
		self.nodes.add_passthrough(theta, parent.end(), 3);

		let mut inner = Link::from(region.start()).iter();

		self.io_state = inner.next().unwrap();
		self.store_state = inner.next().unwrap();
		self.pointer = inner.next().unwrap();

		region.end()
	}

	fn add_block_start(&mut self) {
		let on_false = self.nodes.add_region();
		let on_true = self.nodes.add_region();

		let condition = self.add_load_direct();
		let store_state = self.reconcile_store_state();
		let gamma = self.nodes.add_gamma([on_false, on_true].into());

		self.nodes[gamma].as_parameters_mut().unwrap().extend([
			self.io_state,
			store_state,
			self.pointer,
			condition,
		]);

		self.nodes.add_identity_handle(on_false);

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

		self.nodes[block.output]
			.as_parameters_mut()
			.unwrap()
			.extend([self.io_state, store_state, self.pointer, condition]);

		let mut iter = Link::from(block.parent).iter();

		self.io_state = iter.next().unwrap();
		self.store_state = iter.next().unwrap();
		self.pointer = iter.next().unwrap();

		Ok(())
	}

	fn initialize_nodes(&mut self) {
		// Graph may be in an invalid state if parsing fails.
		let mut nodes = Nodes::new();

		self.io_state = nodes.add_simple(Simple::IO).into();
		self.store_state = nodes.add_simple(Simple::Memory).into();
		self.pointer = nodes.add_integer(0);

		self.nodes = nodes;
		self.blocks.clear();
	}

	/// # Errors
	///
	/// Returns `ParseError::TooManyClosingBrackets` if there are more closing brackets than opening brackets.
	/// Returns `ParseError::TooLittleClosingBrackets` if there are more opening brackets than closing brackets.
	pub fn parse(&mut self, source: CharIndices) -> Result<ParseData, ParseError> {
		self.initialize_nodes();

		for (i, c) in source {
			match c {
				'>' => self.add_pointer_shift(|lhs, rhs| Simple::Add { lhs, rhs }),
				'<' => self.add_pointer_shift(|lhs, rhs| Simple::Sub { lhs, rhs }),
				'+' => self.add_memory_arithmetic(|lhs, rhs| Simple::Add { lhs, rhs }),
				'-' => self.add_memory_arithmetic(|lhs, rhs| Simple::Sub { lhs, rhs }),
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

		let nodes = std::mem::take(&mut self.nodes);
		let io = self.io_state.node;

		Ok(ParseData { nodes, io })
	}
}
