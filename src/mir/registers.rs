use regioned::{
	data_flow::{
		link::{Id, Link},
		node::Parameters,
	},
	visit::{reverse_topological::ReverseTopological, successors::Successors},
};

use crate::hir::data::Nodes;

#[derive(Default)]
pub struct ResultMap {
	results: Vec<Vec<u32>>,
}

impl ResultMap {
	fn reset(&mut self, nodes: &Nodes) {
		let active = nodes.active();

		self.results.iter_mut().for_each(Vec::clear);

		if self.results.len() < active {
			self.results.resize_with(active, Vec::new);
		}
	}

	pub fn get(&self, link: Link) -> u32 {
		let index = usize::from(link.port);

		self.results[link.node][index]
	}

	pub fn set(&mut self, link: Link, register: u32) {
		let index = usize::from(link.port);
		let list = &mut self.results[link.node];

		if index >= list.len() {
			list.resize(index + 1, 0);
		}

		list[index] = register;
	}
}

#[derive(Default)]
pub struct Registers {
	successors: Successors,

	assigned: ResultMap,
	remaining: Vec<usize>,
}

impl Registers {
	pub fn reset<I>(&mut self, nodes: &Nodes, roots: I, topological: &mut ReverseTopological)
	where
		I: IntoIterator<Item = Id>,
	{
		self.successors.run(nodes, roots, topological);
		self.assigned.reset(nodes);
		self.remaining.clear();
	}

	pub const fn assigned(&self) -> &ResultMap {
		&self.assigned
	}

	pub fn register_count(&self) -> usize {
		self.remaining.len()
	}

	fn references_count(&self, nodes: &Nodes, value: Link) -> usize {
		let successors = self.successors.cache()[value.node].iter();

		successors
			.flat_map(|&id| nodes[id].parameters())
			.filter(|&&link| link == value)
			.count()
	}

	fn next_available(&mut self) -> u32 {
		let inactive = self.remaining.iter().position(|&count| count == 0);
		let register = inactive.unwrap_or_else(|| {
			self.remaining.push(0);
			self.remaining.len() - 1
		});

		register.try_into().unwrap()
	}

	pub fn fetch(&mut self, link: Link) -> u32 {
		let register = self.assigned.get(link);
		let index = usize::try_from(register).unwrap();

		self.remaining[index] -= 1;

		register
	}

	pub fn reuse(&mut self, nodes: &Nodes, link: Link, register: u32) {
		let index = usize::try_from(register).unwrap();

		self.assigned.set(link, register);

		self.remaining[index] += self.references_count(nodes, link);
	}

	pub fn reserve(&mut self, nodes: &Nodes, link: Link) -> u32 {
		let register = self.next_available();

		self.reuse(nodes, link, register);

		register
	}

	pub fn reuse_or_reserve(&mut self, nodes: &Nodes, link: Link, preferred: u32) -> u32 {
		let index = usize::try_from(preferred).unwrap();

		if self.remaining[index] == 0 {
			self.reuse(nodes, link, preferred);

			preferred
		} else {
			self.reserve(nodes, link)
		}
	}
}
