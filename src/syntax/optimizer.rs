use super::shared::Bop;

fn fold_loop_list(norm: &mut Vec<Bop>) {
	for v in norm {
		if let Bop::Loop(w) = v {
			fold_bf_code(w);
		}
	}
}

fn fold_action_list(norm: &mut Vec<Bop>) {
	let mut to_remove = Vec::new();

	for i in 0..norm.len() - 1 {
		let a = &norm[i];
		let b = &norm[i + 1];

		if let Some(new) = a.merge(b) {
			to_remove.push(i);
			norm[i + 1] = new;
		}
	}

	for i in to_remove.iter().rev() {
		norm.remove(*i);
	}
}

pub fn fold_bf_code(norm: &mut Vec<Bop>) {
	let mut len = norm.len();

	fold_loop_list(norm);

	loop {
		fold_action_list(norm);

		if len == norm.len() {
			break;
		}

		len = norm.len();
	}

	norm.shrink_to_fit();
}
