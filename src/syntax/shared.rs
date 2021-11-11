#[derive(Clone)]
pub enum Bop {
	Loop(Vec<Bop>),
	DataPointer(i32),
	DataValue(i32),
	Output(u32),
	Input(u32),
}

impl Bop {
	pub fn merge(&self, other: &Self) -> Option<Self> {
		let result = match (self, other) {
			(Self::DataPointer(a), Self::DataPointer(b)) => Self::DataPointer(a + b),
			(Self::DataValue(a), Self::DataValue(b)) => Self::DataValue(a + b),
			(Self::Output(a), Self::Output(b)) => Self::Output(a + b),
			(Self::Input(a), Self::Input(b)) => Self::Input(a + b),
			_ => {
				return None;
			}
		};

		Some(result)
	}
}
