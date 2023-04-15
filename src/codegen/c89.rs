use std::io::{Result, Write};

use crate::mir::data::{Instruction, Program};

use super::tab::Tab;

static MEMORY_SIZE: usize = 8192;
static MEMORY_START: usize = MEMORY_SIZE / 2;

fn write_insn(
	w: &mut dyn Write,
	tab: Tab,
	bodies: &[Box<[Instruction]>],
	insn: &Instruction,
) -> Result<()> {
	match insn {
		Instruction::Memory { result } => {
			writeln!(w, "loc_{result} = {MEMORY_START};")
		}
		Instruction::IO { result } => {
			writeln!(w, "loc_{result} = 0; /* io state is no-op in C */")
		}
		Instruction::Integer { result, value } => writeln!(w, "loc_{result} = {value};"),
		Instruction::Move { from, to } => writeln!(w, "loc_{to} = loc_{from};"),
		Instruction::Add { result, lhs, rhs } => {
			writeln!(w, "loc_{result} = loc_{lhs} + loc_{rhs};")
		}
		Instruction::Sub { result, lhs, rhs } => {
			writeln!(w, "loc_{result} = loc_{lhs} - loc_{rhs};")
		}
		Instruction::Load {
			result,
			pointer,
			state,
		} => {
			writeln!(w, "loc_{result} = memory[loc_{pointer} + loc_{state}];")
		}
		Instruction::Store {
			pointer,
			value,
			state,
		} => {
			writeln!(w, "memory[loc_{pointer} + loc_{state}] = loc_{value};")
		}
		Instruction::Ask { result, .. } => {
			writeln!(w, "loc_{result} = fgetc(stdin);")
		}
		Instruction::Tell { value, .. } => {
			writeln!(w, "fputc(loc_{value}, stdout);")
		}
		Instruction::Select { condition, code } => {
			let mut iter = code.iter();
			let last = iter.next_back().unwrap();

			writeln!(w, "switch (loc_{condition}) {{")?;

			for (i, code) in iter.enumerate() {
				writeln!(w, "{tab}case {i}:")?;
				write_block(w, tab.add(), bodies, *code)?;
				writeln!(w, "{tab}break;")?;
			}

			writeln!(w, "{tab}default:")?;
			write_block(w, tab.add(), bodies, *last)?;
			writeln!(w, "{tab}}}")
		}
		Instruction::Repeat { code, condition } => {
			writeln!(w, "do {{")?;

			write_block(w, tab.add(), bodies, *code)?;

			writeln!(w, "{tab}}} while (loc_{condition});")
		}
	}
}

fn write_block(
	w: &mut dyn Write,
	tab: Tab,
	bodies: &[Box<[Instruction]>],
	index: usize,
) -> Result<()> {
	bodies[index].iter().try_for_each(|insn| {
		write!(w, "{tab}")?;

		write_insn(w, tab, bodies, insn)
	})
}

fn write_entry(w: &mut dyn Write, tab: Tab, program: &Program) -> Result<()> {
	writeln!(w, "{tab}uint8_t memory[{MEMORY_SIZE}] = {{ 0 }};")?;

	for index in 0..program.locals() {
		writeln!(w, "{tab}uint32_t loc_{index};")?;
	}

	write_block(w, tab, program.bodies(), 0)?;

	writeln!(w, "{tab}return 0;")
}

/// # Errors
///
/// Returns an error if the writer fails.
pub fn write(writer: &mut dyn Write, program: &Program) -> Result<()> {
	writeln!(writer, "#include <stdint.h>")?;
	writeln!(writer, "#include <stdio.h>\n")?;

	writeln!(writer, "int main() {{")?;

	write_entry(writer, Tab::new(1), program)?;

	writeln!(writer, "}}")
}
