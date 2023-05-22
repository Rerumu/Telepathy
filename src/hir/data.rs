use std::io::{Result, Write};

use regioned::{
	data_flow::{
		link::{Id, Link, Region},
		node::{AsParametersMut, Parameters, ParametersMut},
	},
	dot::Description,
};

pub enum Simple {
	NoOp,

	Merge {
		states: Vec<Link>,
	},
	Memory,
	IO,

	Integer {
		value: u64,
	},

	Add {
		lhs: Link,
		rhs: Link,
	},
	Sub {
		lhs: Link,
		rhs: Link,
	},

	Load {
		state: Link,
		pointer: Link,
	},
	Store {
		state: Link,
		pointer: Link,
		value: Link,
	},

	Ask {
		state: Link,
	},
	Tell {
		state: Link,
		value: Link,
	},
}

impl Simple {
	const fn name(&self) -> &'static str {
		match self {
			Self::NoOp => "NoOp",
			Self::Merge { .. } => "Merge",
			Self::Memory => "Memory",
			Self::IO => "IO",
			Self::Integer { .. } => "Integer",
			Self::Add { .. } => "Add",
			Self::Sub { .. } => "Sub",
			Self::Load { .. } => "Load",
			Self::Store { .. } => "Store",
			Self::Ask { .. } => "Ask",
			Self::Tell { .. } => "Tell",
		}
	}
}

impl AsParametersMut for Simple {
	fn as_parameters_mut(&mut self) -> Option<&mut Vec<Link>> {
		None
	}
}

// FIXME: These should use custom iterators that avoid allocating.
impl Parameters for Simple {
	type Iter<'a> = std::vec::IntoIter<&'a Link>;

	fn parameters(&self) -> Self::Iter<'_> {
		let results = match self {
			Self::NoOp | Self::Memory | Self::IO | Self::Integer { .. } => Vec::new(),
			Self::Merge { states } => states.iter().collect(),
			Self::Add { lhs, rhs } | Self::Sub { lhs, rhs } => vec![lhs, rhs],
			Self::Load { state, pointer } => vec![state, pointer],
			Self::Store {
				state,
				pointer,
				value,
			} => vec![state, pointer, value],
			Self::Ask { state } => vec![state],
			Self::Tell { state, value } => vec![state, value],
		};

		results.into_iter()
	}
}

impl ParametersMut for Simple {
	type Iter<'a> = std::vec::IntoIter<&'a mut Link>;

	fn parameters_mut(&mut self) -> Self::Iter<'_> {
		let results = match self {
			Self::NoOp | Self::Memory | Self::IO | Self::Integer { .. } => Vec::new(),
			Self::Merge { states } => states.iter_mut().collect(),
			Self::Add { lhs, rhs } | Self::Sub { lhs, rhs } => vec![lhs, rhs],
			Self::Load { state, pointer } => vec![state, pointer],
			Self::Store {
				state,
				pointer,
				value,
			} => vec![state, pointer, value],
			Self::Ask { state } => vec![state],
			Self::Tell { state, value } => vec![state, value],
		};

		results.into_iter()
	}
}

impl Description for Simple {
	fn write_content(&self, writer: &mut dyn Write) -> Result<()> {
		write!(writer, "{}", self.name())
	}
}

impl From<Simple> for Node {
	fn from(value: Simple) -> Self {
		Self::Simple(value)
	}
}

pub type Node = regioned::data_flow::node::Node<Simple>;

pub type Nodes = regioned::data_flow::nodes::Nodes<Simple>;

pub trait Builder {
	fn add_integer(&mut self, value: u64) -> Link;

	fn add_passthrough(&mut self, start: Id, end: Id, len: usize);

	fn add_identity_handle(&mut self, parent: Region);
}

impl Builder for Nodes {
	fn add_integer(&mut self, value: u64) -> Link {
		self.add_simple(Simple::Integer { value }).into()
	}

	fn add_passthrough(&mut self, start: Id, end: Id, len: usize) {
		let iter = Link::from(start).iter().take(len);

		self[end].as_parameters_mut().unwrap().extend(iter);
	}

	fn add_identity_handle(&mut self, parent: Region) {
		self.add_passthrough(parent.start(), parent.end(), 3);
	}
}
