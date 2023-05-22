#[allow(
	clippy::collapsible_match,
	clippy::match_ref_pats,
	clippy::pedantic,
	unreachable_patterns,
	unused_imports,
	unused_variables
)]
mod internal {
	use regioned::data_flow::{link::Link, node::Parameters};

	use crate::hir::data::{Builder, Nodes, Simple};

	include!(concat!(env!("OUT_DIR"), "/isle_internal.rs"));

	impl From<Math> for Simple {
		fn from(value: Math) -> Self {
			match value {
				Math::Integer { value } => Self::Integer { value },
				Math::Add { lhs, rhs } => Self::Add { lhs, rhs },
				Math::Sub { lhs, rhs } => Self::Sub { lhs, rhs },
			}
		}
	}

	impl Context for Nodes {
		fn fold_add(&mut self, lhs: u64, rhs: u64) -> u64 {
			lhs.wrapping_add(rhs)
		}

		fn fold_sub(&mut self, lhs: u64, rhs: u64) -> u64 {
			lhs.wrapping_sub(rhs)
		}

		fn fetch_solo_state(&mut self, link: Link) -> Option<Link> {
			let mut predecessors = self[link.node].parameters();
			let first = predecessors.next().copied();

			first.filter(|first| predecessors.all(|node| node == first))
		}

		fn link_to_math(&mut self, link: Link) -> Option<Math> {
			self[link.node].as_simple().and_then(|node| {
				let node = match *node {
					Simple::Integer { value } => Math::Integer { value },
					Simple::Add { lhs, rhs } => Math::Add { lhs, rhs },
					Simple::Sub { lhs, rhs } => Math::Sub { lhs, rhs },
					_ => return None,
				};

				Some(node)
			})
		}

		fn math_to_link(&mut self, node: &Math) -> Link {
			self.add_simple(node.clone().into()).into()
		}

		fn link_to_memory(&mut self, link: Link) -> Option<Memory> {
			self[link.node].as_simple().and_then(|node| {
				let node = match *node {
					Simple::Merge { .. } => Memory::Merge,
					Simple::Load { state, pointer } => Memory::Load { state, pointer },
					Simple::Store {
						state,
						pointer,
						value,
					} => Memory::Store {
						state,
						pointer,
						value,
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
