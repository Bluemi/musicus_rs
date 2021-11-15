use std::path::{Path, PathBuf};
use std::fs::DirEntry;
use std::fs;

pub fn get_dir_entries(path: &Path) -> Vec<DirectoryEntry> {
	let mut entries = Vec::new();
	if path.is_file() {
		entries.push(DirectoryEntry {
			is_file: true,
			filename: String::from(path.file_name().unwrap().to_str().unwrap()),
			path: PathBuf::from(path),
		});
		return entries;
	}
	if let Ok(read_dir) = path.read_dir() {
		for entry in read_dir.flatten() {
			let entry = DirectoryEntry::from(entry);
			if !entry.filename.starts_with('.') {
				entries.push(entry);
			}
		}
	}
	entries.sort();
	entries
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Debug)]
pub struct DirectoryEntry {
	pub is_file: bool,
	pub filename: String,
	pub path: PathBuf,
}

impl DirectoryEntry {
	pub fn is_song_file(&self) -> bool {
		self.is_file && (self.filename.ends_with(".wav") || self.filename.ends_with(".mp3") || self.filename.ends_with(".ogg"))
	}
}

impl From<DirEntry> for DirectoryEntry {
	fn from(dir_entry: DirEntry) -> Self {
		DirectoryEntry {
			filename: dir_entry.file_name().into_string().unwrap(),
			is_file: dir_entry.file_type().map_or(true, |de| de.is_file()),
			path: dir_entry.path(),
		}
	}
}

pub fn get_common_ends_of_strings<'a>(name: &'a str, begin: &'a str, end: &'a str) -> (&'a str, &'a str) {
	// search for start
	let mut new_begin = begin;
	for (name_char, begin_char) in name.char_indices().zip(begin.char_indices()) {
		if name_char.1 != begin_char.1 {
			new_begin = &name[0..(name_char.0)];
			break;
		}
	}

	// search for end
	let mut new_end = end;
	let mut last_end_index = name.len();
	for (name_char, end_char) in name.char_indices().rev().zip(end.char_indices().rev()) {
		if name_char.1 != end_char.1 {
			new_end = &name[last_end_index..];
			break;
		} else {
			last_end_index = name_char.0;
		}
	}

	(new_begin, new_end)
}

pub fn get_common_ends<'a, I>(strings: I) -> Option<(&'a str, &'a str)>
	where I: IntoIterator<Item = &'a str>
{
	let mut strings = strings.into_iter();
	if let Some(first) = strings.next() {
		let (mut begin, mut end) = (first, first);

		for entry in strings {
			(begin, end) = get_common_ends_of_strings(entry, begin, end);
		}

		Some((begin, end))
	} else {
		None
	}
}

pub fn create_dir(path: &Path) {
	if !path.is_dir() {
		fs::create_dir(path).unwrap();
	}
}

pub fn normalize_dir(path: &mut PathBuf) {
	while !path.is_dir() {
		path.pop();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_common_ends() {
		let a = "startINBETWEENend";
		let b = "startSOMETHINGELSEend";
		let (start, end) = get_common_ends_of_strings(a, b, b);
		assert_eq!(start, "start");
		assert_eq!(end, "end");
	}

	#[test]
	fn test_multiple_ends1() {
		let vec = vec![
			"startfurtherXfurtherend",
			"startfurtherYfurtherend",
			"startZend",
		];
		let (begin, end) = get_common_ends(vec).unwrap();
		assert_eq!(begin, "start");
		assert_eq!(end, "end");
	}

	#[test]
	fn test_multiple_ends2() {
		let vec = vec![];
		assert!(get_common_ends(vec).is_none());
	}

	#[test]
	fn test_multiple_ends3() {
		let vec = vec![
			"abc"
		];
		let (begin, end) = get_common_ends(vec).unwrap();
		assert_eq!(begin, "abc");
		assert_eq!(end, "abc");
	}

	#[test]
	fn test_multiple_ends4() {
		let vec = vec![
			"01abcEND",
			"02defEND",
			"11ghiEND",
			"12jklEND",
		];
		let (begin, end) = get_common_ends(vec).unwrap();
		assert_eq!(begin, "");
		assert_eq!(end, "END");
	}

	#[test]
	fn test_common_ends_utf8() {
		let vec = vec![
			"èŷß",
			"èôß"
		];

		let (begin, end) = get_common_ends(vec).unwrap();
		assert_eq!(begin, "è");
		assert_eq!(end, "ß");
	}
}