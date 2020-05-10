use super::shared::Bop;
use std::ops::AddAssign;

fn fold_consecutive<I, N, B>(
	opt: &mut Vec<Bop>,
	norm: &[Bop],
	mut index: usize,
	as_num: N,
	as_bop: B,
) -> usize
where
	I: AddAssign<I> + PartialEq + Default,
	N: Fn(&Bop) -> Option<I>,
	B: FnOnce(I) -> Bop,
{
	let mut num = I::default();

	while let Some(x) = norm.get(index) {
		if let Some(i) = as_num(x) {
			num += i;
			index += 1;
		} else {
			break;
		}
	}

	if num != I::default() {
		opt.push(as_bop(num));
	}

	index
}

fn fold_loop(opt: &mut Vec<Bop>, norm: &[Bop], index: usize) -> usize {
	let sub_opt = fold_instruction_list(norm);

	opt.push(Bop::Loop(sub_opt));
	index + 1
}

// This is not supposed to be good; it will fail to optimize
// code like `+<>-` to a no-op when it should.
pub fn fold_instruction_list(norm: &[Bop]) -> Vec<Bop> {
	let mut opt = Vec::new();
	let mut index = 0;

	loop {
		index = match norm.get(index) {
			Some(Bop::Loop(lp)) => fold_loop(&mut opt, lp.as_ref(), index),
			Some(Bop::DataPointer(_)) => fold_consecutive(
				&mut opt,
				norm,
				index,
				|v| {
					if let &Bop::DataPointer(i) = v {
						Some(i)
					} else {
						None
					}
				},
				Bop::DataPointer,
			),
			Some(Bop::DataValue(_)) => fold_consecutive(
				&mut opt,
				norm,
				index,
				|v| {
					if let &Bop::DataValue(i) = v {
						Some(i)
					} else {
						None
					}
				},
				Bop::DataValue,
			),
			Some(Bop::Output(_)) => fold_consecutive(
				&mut opt,
				norm,
				index,
				|v| {
					if let &Bop::Output(i) = v {
						Some(i)
					} else {
						None
					}
				},
				Bop::Output,
			),
			Some(Bop::Input(_)) => fold_consecutive(
				&mut opt,
				norm,
				index,
				|v| {
					if let &Bop::Input(i) = v {
						Some(i)
					} else {
						None
					}
				},
				Bop::Input,
			),
			None => break,
		}
	}

	opt
}
