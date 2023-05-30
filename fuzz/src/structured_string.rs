use std::ops::Deref;

use arbitrary::{Arbitrary, Result, Unstructured};

#[derive(Arbitrary)]
enum Symbol {
	Add,
	Sub,
	LessThan,
	GreaterThan,
	Comma,
	Dot,
}

impl From<Symbol> for char {
	fn from(value: Symbol) -> Self {
		match value {
			Symbol::Add => '+',
			Symbol::Sub => '-',
			Symbol::LessThan => '<',
			Symbol::GreaterThan => '>',
			Symbol::Comma => ',',
			Symbol::Dot => '.',
		}
	}
}

fn add_open_bracket(remaining: &mut usize, open: &mut usize, buffer: &mut String) {
	*remaining -= 1;
	*open += 1;
	buffer.push('[');
}

fn add_close_bracket(open: &mut usize, buffer: &mut String) {
	*open -= 1;
	buffer.push(']');
}

fn add_code_segment(u: &mut Unstructured, buffer: &mut String) -> Result<()> {
	let len = u.arbitrary_len::<Symbol>()?;

	buffer.reserve(len);

	for _ in 0..len {
		let element = Symbol::arbitrary(u)?;

		buffer.push(element.into());
	}

	Ok(())
}

#[derive(Debug)]
pub struct StructuredString {
	content: String,
}

impl Arbitrary<'_> for StructuredString {
	fn arbitrary(u: &mut Unstructured) -> Result<Self> {
		let mut remaining = u.arbitrary_len::<Vec<Symbol>>()?;
		let mut open = 0;
		let mut content = String::new();

		while !u.is_empty() {
			add_code_segment(u, &mut content)?;

			match (remaining, open) {
				(0, 0) => break,
				(0, _) => add_close_bracket(&mut open, &mut content),
				(_, 0) => add_open_bracket(&mut remaining, &mut open, &mut content),
				(_, _) => {
					if u.arbitrary()? {
						add_close_bracket(&mut open, &mut content);
					} else {
						add_open_bracket(&mut remaining, &mut open, &mut content);
					}
				}
			}
		}

		Ok(Self { content })
	}
}

impl Deref for StructuredString {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.content
	}
}
