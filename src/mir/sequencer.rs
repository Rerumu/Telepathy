use std::collections::HashMap;

use regioned::{
	data_flow::{
		link::{Id, Link, Region},
		node::{Compound, Marker, Parameters},
	},
	visit::reverse_topological::ReverseTopological,
};

use crate::hir::{
	data::{Node, Nodes, Simple},
	parser::ParseData,
};

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

	fn reset<I>(&mut self, nodes: &Nodes, roots: I, topological: &mut ReverseTopological)
	where
		I: IntoIterator<Item = Id>,
	{
		self.bodies.push(Vec::new());
		self.regions.clear();
		self.regions.push(0);
		self.registers.reset(nodes, roots, topological);
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

	fn add_simple(&mut self, simple: &Simple, nodes: &Nodes, id: Id) {
		let mut results = Link::from(id).iter();
		let first = results.next().unwrap();

		match *simple {
			Simple::NoOp => {}
			Simple::Merge { ref states } => {
				let iter = states.iter().rev().map(|link| self.registers.fetch(*link));

				let result = iter.last().unwrap();
				let post = self.registers.reuse_or_reserve(nodes, first, result);

				self.try_add_move(result, post);
			}
			Simple::Memory => {
				let result = self.registers.reserve(nodes, first);

				self.add(Instruction::Memory { result });
			}
			Simple::IO => {
				let result = self.registers.reserve(nodes, first);

				self.add(Instruction::IO { result });
			}
			Simple::Integer { value } => {
				let result = self.registers.reserve(nodes, first);

				self.add(Instruction::Integer { result, value });
			}
			Simple::Add { lhs, rhs } => {
				let lhs = self.registers.fetch(lhs);
				let rhs = self.registers.fetch(rhs);
				let result = self.registers.reserve(nodes, first);

				self.add(Instruction::Add { result, lhs, rhs });
			}
			Simple::Sub { lhs, rhs } => {
				let lhs = self.registers.fetch(lhs);
				let rhs = self.registers.fetch(rhs);
				let result = self.registers.reserve(nodes, first);

				self.add(Instruction::Sub { result, lhs, rhs });
			}
			Simple::Load { state, pointer } => {
				let state = self.registers.fetch(state);
				let post = self.registers.reuse_or_reserve(nodes, first, state);
				let pointer = self.registers.fetch(pointer);
				let result = self.registers.reserve(nodes, results.next().unwrap());

				self.try_add_move(state, post);

				self.add(Instruction::Load {
					result,
					pointer,
					state,
				});
			}
			Simple::Store {
				state,
				pointer,
				value,
			} => {
				let state = self.registers.fetch(state);
				let pointer = self.registers.fetch(pointer);
				let value = self.registers.fetch(value);

				self.add(Instruction::Store {
					pointer,
					value,
					state,
				});

				let post = self.registers.reuse_or_reserve(nodes, first, state);

				self.try_add_move(state, post);
			}
			Simple::Ask { state } => {
				let state = self.registers.fetch(state);
				let post = self.registers.reuse_or_reserve(nodes, first, state);
				let result = self.registers.reserve(nodes, results.next().unwrap());

				self.try_add_move(state, post);

				self.add(Instruction::Ask { result, state });
			}
			Simple::Tell { state, value } => {
				let state = self.registers.fetch(state);
				let value = self.registers.fetch(value);

				self.add(Instruction::Tell { value, state });

				let post = self.registers.reuse_or_reserve(nodes, first, state);

				self.try_add_move(state, post);
			}
		}
	}

	fn add_start_marker(&mut self, nodes: &Nodes, id: Id, parent: Id) {
		match &nodes[parent] {
			// `Gamma` requires for the first region we reserve all result registers. Later, the
			// registers are used to join all the results.
			Node::Compound(Compound::Gamma {
				parameters,
				regions,
			}) => {
				if regions[0].start() == id {
					// Discard all predecessor references.
					for link in parameters {
						self.registers.fetch(*link);
					}

					let results = nodes[regions[0].end()].parameters().len();

					// Reserve as many registers as there are results.
					for link in Link::from(parent).iter().take(results) {
						self.registers.reserve(nodes, link);
					}
				}

				// Reuse input registers directly for each region.
				Link::from(id).iter().zip(parameters).for_each(|entry| {
					let predecessor = self.registers.assigned().get(*entry.1);

					self.registers.reuse(nodes, entry.0, predecessor);
				});
			}
			// `Theta` likewise requires we reserve all result registers. However, these are also the
			// input registers and as such must be moved before the first iteration.
			Node::Compound(Compound::Theta { parameters, .. }) => {
				let ends = Link::from(parent).iter();
				let iter = Link::from(id).iter().zip(parameters).zip(ends);

				// Discard predecessor, reserve matching result, reuse the register as input,
				// and move the input there.
				iter.for_each(|(entry, end)| {
					let from = self.registers.fetch(*entry.1);
					let to = self.registers.reuse_or_reserve(nodes, entry.0, from);

					self.registers.reuse(nodes, end, to);

					self.try_add_move(from, to);
				});
			}
			_ => unreachable!(),
		}

		self.regions.push(self.bodies.len());
		self.bodies.push(Vec::new());
	}

	fn add_end_marker(&mut self, nodes: &Nodes, parameters: &[Link], parent: Id) {
		let mut parameters = parameters.iter();

		if matches!(nodes[parent], Node::Compound(Compound::Theta { .. })) {
			// Do not use the last predecessor, which is the condition.
			parameters.next_back();
		}

		for (to, from) in Link::from(parent).iter().zip(parameters) {
			let to = self.registers.assigned().get(to);
			let from = self.registers.fetch(*from);

			self.try_add_move(from, to);
		}
	}

	fn add_marker(&mut self, marker: &Marker, nodes: &Nodes, id: Id) {
		let parent = self.parents[&id];

		match marker {
			Marker::Start => self.add_start_marker(nodes, id, parent),
			Marker::End { parameters } => self.add_end_marker(nodes, parameters, parent),
		}
	}

	fn add_gamma(&mut self, parameters: &[Link], regions: &[Region]) {
		let condition = parameters.last().unwrap();
		let condition = self.registers.assigned().get(*condition);

		let regions = regions.len();
		let code = self.regions.drain(self.regions.len() - regions..).collect();

		self.add(Instruction::Select { condition, code });
	}

	fn add_theta(&mut self, nodes: &Nodes, region: Region) {
		let code = self.regions.pop().unwrap();

		let condition = nodes[region.end()].parameters().last().unwrap();
		let condition = self.registers.assigned().get(*condition);

		self.add(Instruction::Repeat { code, condition });
	}

	fn add_compound(&mut self, compound: &Compound, nodes: &Nodes) {
		match compound {
			Compound::Gamma {
				parameters,
				regions,
			} => self.add_gamma(parameters, regions),
			Compound::Theta { region, .. } => self.add_theta(nodes, *region),
			Compound::Lambda { .. } | Compound::Phi { .. } => unreachable!(),
		}
	}

	fn find_parents(&mut self, nodes: &Nodes) {
		self.parents.clear();

		for (id, node) in nodes.iter() {
			if let Node::Compound(compound) = node {
				for region in compound.regions() {
					self.parents.insert(region.start(), id);
					self.parents.insert(region.end(), id);
				}
			}
		}
	}

	#[must_use]
	pub fn sequence(
		&mut self,
		parsed: &ParseData,
		topological: &mut ReverseTopological,
	) -> Program {
		let nodes = parsed.nodes();
		let roots = parsed.roots();

		self.find_parents(nodes);

		self.reset(nodes, roots, topological);

		for id in topological.iter(nodes, roots) {
			match &nodes[id] {
				Node::Simple(simple) => self.add_simple(simple, nodes, id),
				Node::Marker(marker) => self.add_marker(marker, nodes, id),
				Node::Compound(compound) => self.add_compound(compound, nodes),
			}
		}

		let bodies = std::mem::take(&mut self.bodies)
			.into_iter()
			.map(|body| body.into_iter().collect())
			.collect();

		let locals = self.registers.register_count();

		Program::new(bodies, locals)
	}
}
