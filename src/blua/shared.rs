#[derive(Debug)]
pub enum Bop {
	Loop(Vec<Bop>),
	DataPointer(i32),
	DataValue(i32),
	Output(u32),
	Input(u32),
}
