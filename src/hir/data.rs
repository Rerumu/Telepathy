use std::fmt::Debug;

use regioned::{
	data_flow::{
		link::Link,
		node::{Id, Region},
	},
	dot::label::Label,
};

#[derive(Debug, Clone, Copy)]
pub enum Simple {
	NoOp,

	Merge,
	Memory,
	IO,

	Integer(u64),

	Add,
	Sub,

	Load,
	Store,

	Ask,
	Tell,
}

impl Label for Simple {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Debug::fmt(self, f)
	}
}

impl From<Simple> for Node {
	fn from(op: Simple) -> Self {
		Node::Simple(op)
	}
}

pub type Node = regioned::data_flow::node::Node<Simple>;

pub type Graph = regioned::data_flow::graph::Graph<Simple>;

pub trait Builder {
	fn add_single<T>(&mut self, node: T) -> Link
	where
		T: Into<Node>;

	fn add_parametrized<T, I>(&mut self, node: T, params: I) -> Link
	where
		T: Into<Node>,
		I: IntoIterator<Item = Link>;

	fn add_integer(&mut self, value: u64) -> Link;

	fn add_passthrough(&mut self, start: Id, end: Id, len: usize);

	fn add_identity_handle(&mut self, parent: Region);
}

impl Builder for Graph {
	fn add_single<T>(&mut self, node: T) -> Link
	where
		T: Into<Node>,
	{
		self.add_node(node.into()).into()
	}

	fn add_parametrized<T, I>(&mut self, node: T, params: I) -> Link
	where
		T: Into<Node>,
		I: IntoIterator<Item = Link>,
	{
		let link = self.add_single(node);
		let predecessors = params.into_iter().collect();

		self.predecessors[link.node()] = predecessors;

		link
	}

	fn add_integer(&mut self, value: u64) -> Link {
		self.add_single(Simple::Integer(value))
	}

	fn add_passthrough(&mut self, start: Id, end: Id, len: usize) {
		let iter = Link::from(start).iter().take(len);

		self.predecessors[end].extend(iter);
	}

	fn add_identity_handle(&mut self, parent: Region) {
		self.add_passthrough(parent.start(), parent.end(), 3);
	}
}
