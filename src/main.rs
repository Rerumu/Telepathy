use syntax::{
	optimizer::fold_instruction_list,
	parser::{bf_code_to_ast, Error},
	shared::Bop,
};
use std::{fs, io};
use target::{lua, python};

mod syntax;
mod target;

#[derive(Copy, Clone)]
enum Language {
	Lua,
	Python,
}

fn make_ast(source: &str) -> io::Result<Vec<Bop>> {
	bf_code_to_ast(source).map_err(|e| match e {
		Error::NotEof(n) => {
			let info = format!("`eof` expected at index {}", n);

			io::Error::new(io::ErrorKind::Other, info)
		}
		Error::NotClosed(n) => {
			let info = format!("expected `]` to close `[` at index {}", n);

			io::Error::new(io::ErrorKind::Other, info)
		}
		_ => unreachable!(),
	})
}

fn print_file(name: &str, is_opt: bool, lang: Language) -> io::Result<()> {
	let source = fs::read_to_string(name)?;
	let mut ast = make_ast(source.as_str())?;

	if is_opt {
		ast = fold_instruction_list(ast.as_ref());
	}

	let source = match lang {
		Language::Lua => lua::from_ast(ast.as_ref()),
		Language::Python => python::from_ast(ast.as_ref()),
	};

	println!("{}", source);
	Ok(())
}

fn main() -> io::Result<()> {
	let mut is_opt = true;
	let mut lang = Language::Lua;

	for arg in std::env::args().skip(1) {
		match arg.as_str() {
			"--lua" => {
				lang = Language::Lua;
			}
			"--python" => {
				lang = Language::Python;
			}
			"--opt" | "-O" => {
				is_opt = true;
			}
			"--no-opt" | "-N" => {
				is_opt = false;
			}
			name => {
				print_file(name, is_opt, lang)?;
			}
		}
	}

	Ok(())
}
