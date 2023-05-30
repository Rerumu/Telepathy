use std::ops::Deref;

use arbitrary::{Arbitrary, Result, Unstructured};

#[derive(Arbitrary)]
enum Symbol {
	Plus,
	Minus,
	LessThan,
	GreaterThan,
	LeftBracket,
	RightBracket,
	Dot,
	Comma,
}

impl From<Symbol> for char {
	fn from(value: Symbol) -> Self {
		match value {
			Symbol::Plus => '+',
			Symbol::Minus => '-',
			Symbol::LessThan => '<',
			Symbol::GreaterThan => '>',
			Symbol::LeftBracket => '[',
			Symbol::RightBracket => ']',
			Symbol::Dot => '.',
			Symbol::Comma => ',',
		}
	}
}

#[derive(Debug)]
pub struct RestrictedString {
	content: String,
}

impl Arbitrary<'_> for RestrictedString {
	fn arbitrary(u: &mut Unstructured) -> Result<Self> {
		let len = u.arbitrary_len::<Symbol>()?;
		let mut content = String::with_capacity(len);

		for _ in 0..len {
			let element = Symbol::arbitrary(u)?;

			content.push(element.into());
		}

		Ok(Self { content })
	}
}

impl Deref for RestrictedString {
	type Target = str;

	fn deref(&self) -> &Self::Target {
		&self.content
	}
}
