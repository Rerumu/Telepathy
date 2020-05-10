use super::shared::Bop;
use std::{iter::Peekable, str::CharIndices};

pub type BfResult<T> = Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Eof,
	Closed,
	NotEof(usize),
	NotClosed(usize),
}

fn bf_instruction(code: &mut Peekable<CharIndices>) -> BfResult<Bop> {
	loop {
		let value = match code.peek() {
			Some((_, '>')) => Bop::DataPointer(1),
			Some((_, '<')) => Bop::DataPointer(-1),
			Some((_, '+')) => Bop::DataValue(1),
			Some((_, '-')) => Bop::DataValue(-1),
			Some((_, '.')) => Bop::Output(1),
			Some((_, ',')) => Bop::Input(1),
			Some(&(i, '[')) => {
				let body = bf_loop(code, i)?;

				Bop::Loop(body)
			}
			Some((_, ']')) => {
				return Err(Error::Closed);
			}
			None => {
				return Err(Error::Eof);
			}
			_ => {
				code.next();
				continue;
			}
		};

		code.next();
		break Ok(value);
	}
}

fn bf_loop(code: &mut Peekable<CharIndices>, index: usize) -> BfResult<Vec<Bop>> {
	code.next(); // '['

	let block = bf_block(code)?;

	match code.peek() {
		Some((_, ']')) => Ok(block),
		_ => Err(Error::NotClosed(index)),
	}
}

fn bf_block(code: &mut Peekable<CharIndices>) -> BfResult<Vec<Bop>> {
	let mut block = Vec::new();

	loop {
		match bf_instruction(code) {
			Ok(i) => {
				block.push(i);
			}
			Err(Error::NotClosed(i)) => {
				return Err(Error::NotClosed(i));
			}
			Err(_) => {
				break;
			}
		}
	}

	Ok(block)
}

pub fn bf_code_to_ast(code: &str) -> BfResult<Vec<Bop>> {
	let mut it = code.char_indices().peekable();
	let block = bf_block(&mut it)?;

	match it.peek() {
		Some(&(i, _)) => Err(Error::NotEof(i)),
		None => Ok(block),
	}
}
