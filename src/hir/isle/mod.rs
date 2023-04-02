#[allow(
	clippy::collapsible_match,
	clippy::match_ref_pats,
	clippy::pedantic,
	unreachable_patterns,
	unused_imports,
	unused_variables
)]
mod internal {
	use regioned::data_flow::link::Link;

	use crate::hir::data::{Builder, Graph, Simple};

	include!(concat!(env!("OUT_DIR"), "/isle_internal.rs"));

	impl Context for Graph {
		fn fold_add(&mut self, lhs: u64, rhs: u64) -> u64 {
			lhs.wrapping_add(rhs)
		}

		fn fold_sub(&mut self, lhs: u64, rhs: u64) -> u64 {
			lhs.wrapping_sub(rhs)
		}

		fn fetch_solo_state(&mut self, link: Link) -> Option<Link> {
			let mut predecessors = self.predecessors[link.node()].iter();
			let first = predecessors.next().copied();

			first.filter(|first| predecessors.all(|node| node == first))
		}

		fn link_to_math(&mut self, link: Link) -> Option<Math> {
			self.nodes[link.node()].as_simple().and_then(|&node| {
				let predecessors = &self.predecessors[link.node()];
				let node = match node {
					Simple::Integer(value) => Math::Integer { value },
					Simple::Add => Math::Add {
						lhs: predecessors[0],
						rhs: predecessors[1],
					},
					Simple::Sub => Math::Sub {
						lhs: predecessors[0],
						rhs: predecessors[1],
					},
					_ => return None,
				};

				Some(node)
			})
		}

		fn math_to_link(&mut self, node: &Math) -> Link {
			match *node {
				Math::Integer { value } => self.add_single(Simple::Integer(value)),
				Math::Add { lhs, rhs } => self.add_parametrized(Simple::Add, [lhs, rhs]),
				Math::Sub { lhs, rhs } => self.add_parametrized(Simple::Sub, [lhs, rhs]),
			}
		}

		fn link_to_memory(&mut self, link: Link) -> Option<Memory> {
			self.nodes[link.node()].as_simple().and_then(|&node| {
				let predecessors = &self.predecessors[link.node()];
				let node = match node {
					Simple::Merge => Memory::Merge,
					Simple::Load => Memory::Load {
						state: predecessors[0],
						pointer: predecessors[1],
					},
					Simple::Store => Memory::Store {
						state: predecessors[0],
						pointer: predecessors[1],
						value: predecessors[2],
					},
					_ => return None,
				};

				Some(node)
			})
		}
	}
}

pub use internal::{
	constructor_elide as elide, constructor_fold as fold, constructor_identity as identity, Elided,
	Math,
};
