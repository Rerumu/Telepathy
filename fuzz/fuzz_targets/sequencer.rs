#![no_main]

use libfuzzer_sys::{fuzz_target, Corpus};
use regioned::visit::reverse_topological::ReverseTopological;
use telepathy::{hir::parser::Parser, mir::sequencer::Sequencer};
use telepathy_fuzz::structured_string::StructuredString;

fuzz_target!(|source: StructuredString| -> Corpus {
	let Ok(data) = Parser::new().parse(source.char_indices()) else { return Corpus::Reject };

	let mut topological = ReverseTopological::new();
	let _ = Sequencer::new().sequence(&data, &mut topological);

	Corpus::Keep
});
