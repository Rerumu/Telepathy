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

	fn write_arith(&mut self, i: i32, var: &str) {
		let sign = if i < 0 { '-' } else { '+' };
		let value = format!("{0} = {0} {1} {2};", var, sign, i.abs());

		self.write_line(value.as_str());
	}

	fn write_loop(&mut self, code: &[Bop]) {
		self.write_line("while (memory[pointer] != 0) {{");
		self.indent += 1;
		self.write_block(code);
		self.buffer.pop();
		self.indent -= 1;
		self.write_line("}}");
	}

	fn write_output(&mut self, num: u32) {
		if num != 1 {
			let head = format!("for (size_t i = 0; i < {}; i += 1) {{", num);

			self.write_line(head.as_str());
			self.indent += 1;
		}

		self.write_line("putchar(memory[pointer]);");

		if num != 1 {
			self.indent -= 1;
			self.write_line("}");
		}
	}

	fn write_input(&mut self, num: u32) {
		for _ in (0..num).skip(1) {
			self.write_line("getchar();");
		}

		self.write_line("memory[pointer] = getchar();");
	}

	fn write_block(&mut self, code: &[Bop]) {
		for op in code {
			match op {
				Bop::Loop(lp) => self.write_loop(lp),
				&Bop::DataPointer(i) => self.write_arith(i, "pointer"),
				&Bop::DataValue(i) => self.write_arith(i, "memory[pointer]"),
				&Bop::Output(i) => self.write_output(i),
				&Bop::Input(i) => self.write_input(i),
			}

			self.buffer.push('\n');
		}
	}
}

pub fn from_ast(code: &[Bop]) -> String {
	let mut state = State::default();

	state.write_line("#include <stdint.h>");
	state.write_line("#include <stdio.h>");
	state.buffer.push('\n');
	state.write_line("int main(void) {");
	state.indent += 1;
	state.write_line("int32_t memory[8192] = { 0 };");
	state.write_line("size_t pointer = 0;");
	state.buffer.push('\n');
	state.write_block(code);
	state.write_line("return 0;");
	state.indent -= 1;
	state.write_line("}");
	state.buffer
}
