#![no_main]

use libfuzzer_sys::fuzz_target;
use telepathy::hir::parser::Parser;
use telepathy_fuzz::restricted_string::RestrictedString;

fuzz_target!(|source: RestrictedString| {
	let _ = Parser::new().parse(source.char_indices());
});
