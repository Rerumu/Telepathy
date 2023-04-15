use std::collections::HashMap;

use regioned::{
	data_flow::{
		link::Link,
		node::{Compound, Id, Marker},
	},
	visit::reverse_topological::ReverseTopological,
};

use crate::hir::data::{Graph, Node, Simple};

use super::{
	data::{Instruction, Program},
	registers::Registers,
};

#[derive(Default)]
pub struct Sequencer {
	parents: HashMap<Id, Id>,
	regions: Vec<usize>,
	bodies: Vec<Vec<Instruction>>,

	registers: Registers,
}

impl Sequencer {
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	fn reset<I>(&mut self, graph: &Graph, roots: I, topological: &mut ReverseTopological)
	where
		I: IntoIterator<Item = Id>,
	{
		self.bodies.push(Vec::new());
		self.regions.clear();
		self.regions.push(0);
		self.registers.reset(graph, roots, topological);
	}

	fn add(&mut self, instruction: Instruction) {
		let index = self.regions.last().unwrap();

		self.bodies[*index].push(instruction);
	}

	fn try_add_move(&mut self, from: u32, to: u32) {
		if from == to {
			return;
		}

		let instruction = Instruction::Move { from, to };

		self.add(instruction);
	}

	fn add_simple(&mut self, simple: Simple, graph: &Graph, id: Id) {
		let mut results = Link::from(id).iter();
		let first = results.next().unwrap();
		let predecessors = &graph.predecessors[id];

		match simple {
			Simple::NoOp => {}
			Simple::Merge => {
				let iter = predecessors
					.iter()
					.rev()
					.map(|link| self.registers.fetch(*link));

				let result = iter.last().unwrap();
				let post = self.registers.reuse_or_reserve(graph, first, result);

				self.try_add_move(result, post);
			}
			Simple::Memory => {
				let result = self.registers.reserve(graph, first);

				self.add(Instruction::Memory { result });
			}
			Simple::IO => {
				let result = self.registers.reserve(graph, first);

				self.add(Instruction::IO { result });
			}
			Simple::Integer(value) => {
				let result = self.registers.reserve(graph, first);

				self.add(Instruction::Integer { result, value });
			}
			Simple::Add => {
				let lhs = self.registers.fetch(predecessors[0]);
				let rhs = self.registers.fetch(predecessors[1]);
				let result = self.registers.reserve(graph, first);

				self.add(Instruction::Add { result, lhs, rhs });
			}
			Simple::Sub => {
				let lhs = self.registers.fetch(predecessors[0]);
				let rhs = self.registers.fetch(predecessors[1]);
				let result = self.registers.reserve(graph, first);

				self.add(Instruction::Sub { result, lhs, rhs });
			}
			Simple::Load => {
				let state = self.registers.fetch(predecessors[0]);
				let post = self.registers.reuse_or_reserve(graph, first, state);
				let pointer = self.registers.fetch(predecessors[1]);
				let result = self.registers.reserve(graph, results.next().unwrap());

				self.try_add_move(state, post);

				self.add(Instruction::Load {
					result,
					pointer,
					state,
				});
			}
			Simple::Store => {
				let state = self.registers.fetch(predecessors[0]);
				let pointer = self.registers.fetch(predecessors[1]);
				let value = self.registers.fetch(predecessors[2]);

				self.add(Instruction::Store {
					pointer,
					value,
					state,
				});

				let post = self.registers.reuse_or_reserve(graph, first, state);

				self.try_add_move(state, post);
			}
			Simple::Ask => {
				let state = self.registers.fetch(predecessors[0]);
				let post = self.registers.reuse_or_reserve(graph, first, state);
				let result = self.registers.reserve(graph, results.next().unwrap());

				self.try_add_move(state, post);

				self.add(Instruction::Ask { result, state });
			}
			Simple::Tell => {
				let state = self.registers.fetch(predecessors[0]);
				let value = self.registers.fetch(predecessors[1]);

				self.add(Instruction::Tell { value, state });

				let post = self.registers.reuse_or_reserve(graph, first, state);

				self.try_add_move(state, post);
			}
		}
	}

	fn add_start_marker(&mut self, graph: &Graph, id: Id, parent: Id) {
		let predecessors = &graph.predecessors[parent];
		let iter = Link::from(id).iter().zip(predecessors);

		match graph.nodes[parent] {
			// `Gamma` requires for the first region we reserve all result registers. Later, the
			// registers are used to join all the results.
			Node::Compound(Compound::Gamma) => {
				let region = graph.regions[&parent][0];

				if region.start() == id {
					// Discard all predecessor references.
					for link in predecessors {
						self.registers.fetch(*link);
					}

					let results = graph.predecessors[region.end()].len();

					// Reserve as many registers as there are results.
					for link in Link::from(parent).iter().take(results) {
						self.registers.reserve(graph, link);
					}
				}

				// Reuse input registers directly for each region.
				iter.for_each(|entry| {
					let predecessor = self.registers.assigned().get(*entry.1);

					self.registers.reuse(graph, entry.0, predecessor);
				});
			}
			// `Theta` likewise requires we reserve all result registers. However, these are also the
			// input registers and as such must be moved before the first iteration.
			Node::Compound(Compound::Theta) => {
				let ends = Link::from(parent).iter();

				// Discard predecessor, reserve matching result, reuse the register as input,
				// and move the input there.
				iter.zip(ends).for_each(|(entry, end)| {
					let from = self.registers.fetch(*entry.1);
					let to = self.registers.reuse_or_reserve(graph, entry.0, from);

					self.registers.reuse(graph, end, to);

					self.try_add_move(from, to);
				});
			}
			_ => unreachable!(),
		}

		self.regions.push(self.bodies.len());
		self.bodies.push(Vec::new());
	}

	fn add_end_marker(&mut self, graph: &Graph, id: Id, parent: Id) {
		let mut predecessors = graph.predecessors[id].iter();

		if graph.nodes[parent].as_compound() == Some(Compound::Theta) {
			// Do not use the last predecessor, which is the condition.
			predecessors.next_back();
		}

		for (to, from) in Link::from(parent).iter().zip(predecessors) {
			let to = self.registers.assigned().get(to);
			let from = self.registers.fetch(*from);

			self.try_add_move(from, to);
		}
	}

	fn add_marker(&mut self, marker: Marker, graph: &Graph, id: Id) {
		let parent = self.parents[&id];

		match marker {
			Marker::Start => self.add_start_marker(graph, id, parent),
			Marker::End => self.add_end_marker(graph, id, parent),
		}
	}

	fn add_gamma(&mut self, graph: &Graph, id: Id) {
		let condition = graph.predecessors[id].last().unwrap();
		let condition = self.registers.assigned().get(*condition);

		let regions = graph.regions[&id].len();
		let code = self.regions.drain(self.regions.len() - regions..).collect();

		self.add(Instruction::Select { condition, code });
	}

	fn add_theta(&mut self, graph: &Graph, id: Id) {
		let code = self.regions.pop().unwrap();

		let region = graph.regions[&id][0];
		let condition = graph.predecessors[region.end()].last().unwrap();
		let condition = self.registers.assigned().get(*condition);

		self.add(Instruction::Repeat { code, condition });
	}

	fn add_compound(&mut self, compound: Compound, graph: &Graph, id: Id) {
		match compound {
			Compound::Gamma => self.add_gamma(graph, id),
			Compound::Theta => self.add_theta(graph, id),
			Compound::Lambda | Compound::Phi => unreachable!(),
		}
	}

	fn find_parents(&mut self, graph: &Graph) {
		self.parents.clear();

		for (id, node) in &graph.nodes {
			if node.as_compound().is_none() {
				continue;
			}

			for region in &graph.regions[&id] {
				self.parents.insert(region.start(), id);
				self.parents.insert(region.end(), id);
			}
		}
	}

	#[must_use]
	pub fn sequence<I>(
		&mut self,
		graph: &Graph,
		roots: I,
		topological: &mut ReverseTopological,
	) -> Program
	where
		I: IntoIterator<Item = Id> + Clone,
	{
		self.find_parents(graph);

		self.reset(graph, roots.clone(), topological);

		topological.run_with(graph, roots, |graph, id| match graph.nodes[id] {
			Node::Simple(simple) => self.add_simple(simple, graph, id),
			Node::Marker(marker) => self.add_marker(marker, graph, id),
			Node::Compound(compound) => self.add_compound(compound, graph, id),
		});

		let bodies = std::mem::take(&mut self.bodies)
			.into_iter()
			.map(|body| body.into_iter().collect())
			.collect();

		let locals = self.registers.register_count();

		Program::new(bodies, locals)
	}
}
