use crate::syntax::shared::Bop;

#[derive(Default)]
struct State {
	buffer: String,
	indent: usize,
}

impl State {
	fn write_line(&mut self, value: &str) {
		let indent = "\t".repeat(self.indent);

		self.buffer.push_str(indent.as_str());
		self.buffer.push_str(value);
		self.buffer.push('\n');
	}

	fn write_arith(&mut self, i: i32, lt: &str, gt: &str) {
		let sign = if i < 0 { lt } else { gt };
		let data = sign.repeat(i.abs() as usize);

		self.write_line(&data);
	}

	fn write_loop(&mut self, code: &[Bop]) {
		self.write_line("[");
		self.indent += 1;
		self.write_block(code);
		self.indent -= 1;
		self.write_line("]");
	}

	fn write_output(&mut self, num: u32) {
		let data = ".".repeat(num as usize);

		self.write_line(&data);
	}

	fn write_input(&mut self, num: u32) {
		let data = ",".repeat(num as usize);

		self.write_line(&data);
	}

	fn write_block(&mut self, code: &[Bop]) {
		for op in code {
			match op {
				Bop::Loop(lp) => self.write_loop(lp),
				&Bop::DataPointer(i) => self.write_arith(i, "<", ">"),
				&Bop::DataValue(i) => self.write_arith(i, "-", "+"),
				&Bop::Output(i) => self.write_output(i),
				&Bop::Input(i) => self.write_input(i),
			}
		}
	}
}

pub fn from_ast(code: &[Bop]) -> String {
	let mut state = State::default();

	state.write_block(code);

	state.buffer
}
