use std::{fs::File, io::Read, path::PathBuf};

use pocket_topo::parser;

#[test]
fn parses_default() {
	let contents = fixture("default.top");

	let document = parser::parse(&contents).expect("invalid document");

	let mut references = document.references().iter();
	assert_eq!(references.len(), 1);

	let _reference = references.next().expect("expected reference");

	assert!(references.next().is_none());

	assert_eq!(document.trips().len(), 0);

	assert_eq!(document.shots().len(), 1);
}

#[test]
fn parses_empty() {
	let contents = fixture("empty.top");

	let document = parser::parse(&contents).expect("invalid document");

	assert_eq!(document.shots().len(), 0);
	assert_eq!(document.trips().len(), 0);
	assert_eq!(document.references().len(), 0);
}

fn fixture(fixture: &str) -> Vec<u8> {
	let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
	path.push("tests/fixtures");
	path.push(fixture);

	let mut file = File::open(path).unwrap();

	let mut buffer = Vec::new();
	file.read_to_end(&mut buffer).unwrap();

	buffer
}
