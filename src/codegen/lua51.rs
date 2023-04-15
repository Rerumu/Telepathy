use std::io::{Result, Write};

use crate::mir::data::{Instruction, Program};

use super::tab::Tab;

static MEMORY: &str = "setmetatable({}, { __index = function() return 0 end })";
static IO: &str = "{ tell = function(n) io.write(string.char(n)) end, ask = function() return string.byte(io.read(1)) end }";

fn write_insn(
	w: &mut dyn Write,
	tab: Tab,
	bodies: &[Box<[Instruction]>],
	insn: &Instruction,
) -> Result<()> {
	match insn {
		Instruction::Memory { result } => writeln!(w, "loc_{result} = {MEMORY}",),
		Instruction::IO { result } => writeln!(w, "loc_{result} = {IO}"),
		Instruction::Integer { result, value } => writeln!(w, "loc_{result} = {value}"),
		Instruction::Move { from, to } => writeln!(w, "loc_{to} = loc_{from}"),
		Instruction::Add { result, lhs, rhs } => {
			writeln!(w, "loc_{result} = loc_{lhs} + loc_{rhs}")
		}
		Instruction::Sub { result, lhs, rhs } => {
			writeln!(w, "loc_{result} = loc_{lhs} - loc_{rhs}")
		}
		Instruction::Load {
			result,
			pointer,
			state,
		} => {
			writeln!(w, "loc_{result} = loc_{state}[loc_{pointer}]")
		}
		Instruction::Store {
			pointer,
			value,
			state,
		} => {
			writeln!(w, "loc_{state}[loc_{pointer}] = loc_{value}")
		}
		Instruction::Ask { result, state } => {
			writeln!(w, "loc_{result} = loc_{state}.ask()")
		}
		Instruction::Tell { value, state } => {
			writeln!(w, "loc_{state}.tell(loc_{value})")
		}
		Instruction::Select { condition, code } => {
			let mut iter = code.iter();
			let last = iter.next_back().unwrap();

			for (i, code) in iter.enumerate() {
				writeln!(w, "if loc_{condition} == {i} then")?;
				write_block(w, tab.add(), bodies, *code)?;
				write!(w, "{tab}else")?;
			}

			writeln!(w)?;
			write_block(w, tab.add(), bodies, *last)?;
			writeln!(w, "{tab}end")
		}
		Instruction::Repeat { code, condition } => {
			writeln!(w, "repeat")?;

			write_block(w, tab.add(), bodies, *code)?;

			writeln!(w, "{tab}until loc_{condition} == 0")
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

/// # Errors
///
/// Returns an error if the writer fails.
pub fn write(writer: &mut dyn Write, program: &Program) -> Result<()> {
	for index in 0..program.locals() {
		writeln!(writer, "local loc_{index}")?;
	}

	write_block(writer, Tab::new(0), program.bodies(), 0)
}
