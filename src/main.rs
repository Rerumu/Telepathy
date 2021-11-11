use syntax::{
	optimizer::fold_bf_code,
	parser::{bf_code_to_ast, Error},
	shared::Bop,
};

mod syntax;
mod target;

#[derive(Copy, Clone)]
enum Language {
	BrainFuck,
	Lua,
	Python,
}

fn make_ast(source: &str) -> Result<Vec<Bop>, String> {
	bf_code_to_ast(source).map_err(|e| match e {
		Error::NotEof(n) => {
			format!("`eof` expected at index {}", n)
		}
		Error::NotClosed(n) => {
			format!("expected `]` to close `[` at index {}", n)
		}
		_ => unreachable!(),
	})
}

fn translate_to_bf(name: &str, is_opt: bool, lang: Language) -> String {
	let source = std::fs::read_to_string(name).unwrap();
	let mut ast = make_ast(source.as_str()).expect("Failure to translate");

	if is_opt {
		fold_bf_code(ast.as_mut());
	}

	match lang {
		Language::BrainFuck => target::brainfuck::from_ast(ast.as_ref()),
		Language::Lua => target::lua::from_ast(ast.as_ref()),
		Language::Python => target::python::from_ast(ast.as_ref()),
	}
}

fn main() {
	let mut is_opt = true;
	let mut lang = Language::Lua;

	for arg in std::env::args().skip(1) {
		match arg.as_str() {
			"--brainfuck" => {
				lang = Language::BrainFuck;
			}
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
				let bf = translate_to_bf(name, is_opt, lang);

				println!("{}", bf);
			}
		}
	}
}
