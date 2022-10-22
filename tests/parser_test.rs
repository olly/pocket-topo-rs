use std::{fs::File, io::Read, path::PathBuf};

use pocket_topo::{parser, ShotFlags, StationId};

#[test]
fn parses_default() {
	let contents = fixture("default.top");

	let document = parser::parse(&contents).expect("invalid document");

	// Shots
	let mut shots = document.shots.iter();
	assert_eq!(shots.len(), 1);

	let shot = shots.next().unwrap();

	assert_eq!(shot.from, Some(StationId::MajorMinor(1, 0)));
	assert_eq!(shot.to, None);
	assert_eq!(shot.azimuth, 0);
	assert_eq!(shot.distance, 0);
	assert_eq!(shot.inclination, 0);
	assert_eq!(shot.flags, ShotFlags::empty());
	assert!(!shot.flags.contains(ShotFlags::FLIPPED));
	assert!(!shot.flags.contains(ShotFlags::HAS_COMMENT));
	assert_eq!(shot.roll, 0x0);
	assert_eq!(shot.trip_index, -1);

	assert!(shots.next().is_none());

	// Trips
	assert_eq!(document.trips.len(), 0);

	// References
	let mut references = document.references.iter();
	assert_eq!(references.len(), 1);

	let mut _reference = references.next();

	assert!(references.next().is_none());
}

#[test]
fn parses_empty() {
	let contents = fixture("empty.top");

	let document = parser::parse(&contents).expect("invalid document");

	assert_eq!(document.shots.len(), 0);
	assert_eq!(document.trips.len(), 0);
	assert_eq!(document.references.len(), 0);
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
