use crate::blua::shared::Bop;

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
		let value = format!("{} = {} {} {}", var, var, sign, i.abs());

		self.write_line(value.as_str());
	}

	fn write_loop(&mut self, code: &[Bop]) {
		self.write_line("while memory[pointer] ~= 0 do");
		self.indent += 1;
		self.write_block(code);
		self.buffer.pop();
		self.indent -= 1;
		self.write_line("end");
	}

	fn write_output(&mut self, num: u32) {
		if num == 1 {
			self.write_line("output(memory[pointer])");
		} else {
			let head = format!("for _ = 1, {} do", num);

			self.write_line("local temp = memory[pointer]");
			self.write_line(head.as_str());
			self.indent += 1;
			self.write_line("output(temp)");
			self.indent -= 1;
			self.write_line("end");
		}
	}

	fn write_input(&mut self, num: u32) {
		for _ in (0..num).skip(1) {
			self.write_line("input()");
		}

		self.write_line("memory[pointer] = input()");
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

	state.write_line("local memory = setmetatable({}, {__index = function() return 0 end})");
	state.write_line("local pointer = 1");
	state.write_line("local output = function(x) io.write(string.char(x)) end");
	state.write_line("local input = function(x) return io.read(1):byte(1, 1) end");
	state.buffer.push('\n');
	state.write_block(code);
	state.buffer
}
