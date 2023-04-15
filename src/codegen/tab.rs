use std::fmt::{Display, Formatter, Result};

#[derive(Clone, Copy)]
pub struct Tab {
	len: usize,
}

impl Tab {
	pub const fn new(len: usize) -> Self {
		Self { len }
	}

	pub const fn add(self) -> Self {
		Self::new(self.len + 1)
	}
}

impl Display for Tab {
	fn fmt(&self, f: &mut Formatter<'_>) -> Result {
		std::iter::repeat_with(|| write!(f, "\t"))
			.take(self.len)
			.collect()
	}
}
