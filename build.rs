use std::path::Path;

use cranelift_isle::{codegen::CodegenOptions, compile::from_files};

static CODE_OPTIONS: CodegenOptions = CodegenOptions {
	exclude_global_allow_pragmas: true,
};

static FILE_LIST: &[&str] = &["src/hir/isle/rules.isle"];

fn write_to_output(name: &str, contents: &str) {
	let directory = std::env::var("OUT_DIR").unwrap();
	let path = Path::new(&directory).join(name);

	std::fs::write(path, contents).unwrap();
}

fn main() {
	for file in FILE_LIST {
		println!("cargo:rerun-if-changed={file}");
	}

	let code = from_files(FILE_LIST, &CODE_OPTIONS).expect("compilation failed");

	write_to_output("isle_internal.rs", &code);
}
