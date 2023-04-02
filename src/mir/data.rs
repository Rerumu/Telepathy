#[derive(Debug)]
pub struct Program {
	bodies: Box<[Box<[Instruction]>]>,
	locals: usize,
}

impl Program {
	#[must_use]
	pub const fn new(bodies: Box<[Box<[Instruction]>]>, locals: usize) -> Self {
		Self { bodies, locals }
	}

	#[must_use]
	pub fn bodies(&self) -> &[Box<[Instruction]>] {
		&self.bodies
	}

	#[must_use]
	pub const fn locals(&self) -> usize {
		self.locals
	}
}

#[derive(Debug)]
pub enum Instruction {
	Memory {
		result: u32,
	},

	IO {
		result: u32,
	},

	Integer {
		result: u32,
		value: u64,
	},

	Move {
		from: u32,
		to: u32,
	},

	Add {
		result: u32,
		lhs: u32,
		rhs: u32,
	},

	Sub {
		result: u32,
		lhs: u32,
		rhs: u32,
	},

	Load {
		result: u32,
		pointer: u32,
		state: u32,
	},

	Store {
		pointer: u32,
		value: u32,
		state: u32,
	},

	Ask {
		result: u32,
		state: u32,
	},

	Tell {
		value: u32,
		state: u32,
	},

	Select {
		condition: u32,
		code: Box<[usize]>,
	},

	Repeat {
		code: usize,
		condition: u32,
	},
}
