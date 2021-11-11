use std::borrow::Cow;

use super::shared::Bop;

fn fold_instruction_list(norm: &[Bop]) -> Vec<Bop> {
	let mut opt = Vec::with_capacity(norm.len());

	for v in norm {
		if let Bop::Loop(v) = v {
			let inst = fold_bf_code(v);

			opt.push(Bop::Loop(inst));
		} else if let Some(new) = opt.last().and_then(|w| w.merge(v)) {
			*opt.last_mut().unwrap() = new;
		} else {
			opt.push(v.clone());
		}
	}

	opt.shrink_to_fit();
	opt
}

pub fn fold_bf_code(norm: &[Bop]) -> Vec<Bop> {
	let mut result = Cow::Borrowed(norm);

	loop {
		let new = fold_instruction_list(result.as_ref());

		if new.len() == result.len() {
			break;
		}

		result = Cow::Owned(new);
	}

	result.into_owned()
}
