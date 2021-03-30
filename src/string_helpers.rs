pub fn limit_str_right(s: &str, num_visible_chars: usize) -> &str {
	if let Some(i) = s.char_indices().map(|(i, _)| i).nth(num_visible_chars) {
		&s[..i]
	} else {
		&s
	}
}

pub fn cut_str_left(s: &str, num_visible_chars: usize) -> &str {
	if let Some(i) = s.char_indices().map(|(i, _)| i).nth(num_visible_chars) {
		&s[i..]
	} else {
		&s
	}
}

mod tests {
	#[allow(unused_imports)]
	use crate::string_helpers::{limit_str_right, cut_str_left};

	#[test]
	fn test_limit_str_right() {
		let a = "hey";
		assert_eq!(limit_str_right(a, 2), "he");

		let a = "Jürgen";
		assert_eq!(limit_str_right(a, 2), "Jü");
		assert_eq!(limit_str_right(a, 3), "Jür");
		assert_eq!(limit_str_right(a, 6), "Jürgen");
	}

	#[test]
	fn test_cut_str_left() {
		let a = "hey";
		assert_eq!(cut_str_left(a, 1), "ey");

		let a = "Jürgen";
		assert_eq!(cut_str_left(a, 1), "ürgen");
		assert_eq!(cut_str_left(a, 2), "rgen");
	}
}
