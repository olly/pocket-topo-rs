use std::{fs::File, io::Read, path::PathBuf};

use chrono::NaiveDate;
use pocket_topo::{parser, Reference, Shot, ShotFlags, StationId, Trip};

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
	assert_eq!(shot.comment, None);

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

#[test]
fn parses_shots_with_comments() {
	let contents = fixture("comments.top");

	let document = parser::parse(&contents).expect("invalid document");

	// Shots
	let mut shots = document.shots.iter();
	assert_eq!(shots.len(), 2);

	let mut shot: &Shot;

	shot = shots.next().unwrap();
	assert_eq!(shot.from, Some(StationId::MajorMinor(1, 0)));
	assert_eq!(shot.to, Some(StationId::MajorMinor(1, 1)));
	assert_eq!(shot.azimuth, 1820); // 10 deg
	assert_eq!(shot.distance, 123450); // 123.45 m
	assert_eq!(shot.inclination, 5461); // 30 deg
	assert!(!shot.flags.contains(ShotFlags::FLIPPED));
	assert!(shot.flags.contains(ShotFlags::HAS_COMMENT));
	assert_eq!(shot.roll, 0x0);
	assert_eq!(shot.trip_index, -1);
	assert_eq!(
		shot.comment,
		Some("Comment #1\r\n\r\nFrom station: 1.0 to station: 1.1\r\n123.45 / 10.0 / 30,0")
	);

	shot = shots.next().unwrap();
	assert_eq!(shot.from, Some(StationId::MajorMinor(1, 1)));
	assert_eq!(shot.to, Some(StationId::Plain(2)));
	assert_eq!(shot.azimuth, 1220); // 6.7 deg
	assert_eq!(shot.distance, 26340); // 26.340 m
	assert_eq!(shot.inclination, 7719); // 42.4 deg
	assert!(!shot.flags.contains(ShotFlags::FLIPPED));
	assert!(shot.flags.contains(ShotFlags::HAS_COMMENT));
	assert_eq!(shot.roll, 0x0);
	assert_eq!(shot.trip_index, 0);
	assert_eq!(
		shot.comment,
		Some("Comment #2\r\n\r\nfrom station: 1.1 to station 2\r\n26.340 / 6.7 / 42.4")
	);

	assert!(shots.next().is_none());
}

#[test]
fn parses_trips() {
	let contents = fixture("trips.top");

	let document = parser::parse(&contents).expect("invalid document");

	// Trips
	let mut trips = document.trips.iter();
	assert_eq!(trips.len(), 3);

	let mut trip: &Trip;

	trip = trips.next().unwrap();
	assert_eq!(
		trip.time,
		NaiveDate::from_ymd(2022, 10, 22).and_hms(0, 0, 0)
	);
	assert_eq!(trip.comment, "test");
	assert_eq!(trip.declination, 628); // 3.45 deg

	trip = trips.next().unwrap();
	assert_eq!(
		trip.time,
		NaiveDate::from_ymd(2022, 10, 15).and_hms(0, 0, 0)
	);
	assert_eq!(trip.comment, "2022-10-15 2.34");
	assert_eq!(trip.declination, 426); // 2.34 deg

	trip = trips.next().unwrap();
	assert_eq!(
		trip.time,
		NaiveDate::from_ymd(2022, 10, 22).and_hms(0, 0, 0)
	);
	assert_eq!(trip.comment, "2022-10-22 3.45");
	assert_eq!(trip.declination, 628); // 3.45 deg

	assert!(trips.next().is_none());
}

#[test]
fn parses_references() {
	let contents = fixture("references.top");

	let document = parser::parse(&contents).expect("invalid document");

	// References
	let mut references = document.references.iter();
	assert_eq!(references.len(), 3);

	let mut reference: &Reference;

	reference = references.next().unwrap();
	assert_eq!(reference.station, None);
	assert_eq!(reference.east, 24000);
	assert_eq!(reference.north, 42000);
	assert_eq!(reference.altitude, 50000);
	assert_eq!(reference.comment, "");

	reference = references.next().unwrap();
	assert_eq!(reference.station, Some(StationId::MajorMinor(1, 0)));
	assert_eq!(reference.east, 12340);
	assert_eq!(reference.north, 56780);
	assert_eq!(reference.altitude, 90120);
	assert_eq!(reference.comment, "Comment //2\r\n");

	reference = references.next().unwrap();
	assert_eq!(reference.station, None);
	assert_eq!(reference.east, 0);
	assert_eq!(reference.north, 0);
	assert_eq!(reference.altitude, -2147483648);
	assert_eq!(reference.comment, "");

	assert!(references.next().is_none());
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
