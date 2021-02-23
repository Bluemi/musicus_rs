use std::path::{Path, PathBuf};
use std::fs::DirEntry;

pub fn get_dir_entries(path: &Path) -> Vec<DirectoryEntry> {
	let mut entries = Vec::new();
	if let Ok(read_dir) = path.read_dir() {
		for entry in read_dir {
			if let Ok(entry) = entry {
				let entry = DirectoryEntry::from(entry);
				if !entry.filename.starts_with(".") {
					entries.push(entry);
				}
			}
		}
	}
	entries.sort();
	entries
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
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
	let mut slice_index = name.len().min(begin.len());
	let mut new_begin = "";
	while slice_index > 0 {
		if &name[0..slice_index] == &begin[0..slice_index] {
			new_begin = &name[0..slice_index];
			break;
		}
		slice_index -= 1;
	}

	// search for end
	let mut slice_index = name.len().min(end.len()); // equal_end_index defines the number of chars at the end of a string
	let mut new_end = "";
	while slice_index > 0 {
		let name_index = name.len() - slice_index;
		let end_index = end.len() - slice_index;
		if &name[name_index..] == &end[end_index..] {
			new_end = &name[name_index..];
			break;
		}
		slice_index -= 1;
	}
	return (new_begin, new_end);
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
}