use regioned::{
	data_flow::{link::Link, node::Id},
	visit::{reverse_topological::ReverseTopological, successors::Successors},
};

use crate::hir::data::Graph;

#[derive(Default)]
pub struct ResultMap {
	results: Vec<Vec<u32>>,
}

impl ResultMap {
	fn reset(&mut self, graph: &Graph) {
		let active = graph.active();

		self.results.iter_mut().for_each(Vec::clear);

		if self.results.len() < active {
			self.results.resize_with(active, Vec::new);
		}
	}

	pub fn get(&self, link: Link) -> u32 {
		let index = usize::from(link.port().index());

		self.results[link.node()][index]
	}

	pub fn set(&mut self, link: Link, register: u32) {
		let index = usize::from(link.port().index());
		let list = &mut self.results[link.node()];

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
	pub fn reset<I>(&mut self, graph: &Graph, roots: I, topological: &mut ReverseTopological)
	where
		I: IntoIterator<Item = Id>,
	{
		self.successors.run(graph, roots, topological);
		self.assigned.reset(graph);
		self.remaining.clear();
	}

	pub fn assigned(&self) -> &ResultMap {
		&self.assigned
	}

	pub fn register_count(&self) -> usize {
		self.remaining.len()
	}

	fn references_count(&self, graph: &Graph, value: Link) -> usize {
		let successors = self.successors.cache()[value.node()].iter();

		successors
			.flat_map(|&id| &graph.predecessors[id])
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

	pub fn reuse(&mut self, graph: &Graph, link: Link, register: u32) {
		let index = usize::try_from(register).unwrap();

		self.assigned.set(link, register);

		self.remaining[index] += self.references_count(graph, link);
	}

	pub fn reserve(&mut self, graph: &Graph, link: Link) -> u32 {
		let register = self.next_available();

		self.reuse(graph, link, register);

		register
	}

	pub fn reuse_or_reserve(&mut self, graph: &Graph, link: Link, preferred: u32) -> u32 {
		let index = usize::try_from(preferred).unwrap();

		if self.remaining[index] == 0 {
			self.reuse(graph, link, preferred);

			preferred
		} else {
			self.reserve(graph, link)
		}
	}
}
