use std::{fs::File, io::Read, path::PathBuf};

use chrono::NaiveDate;
use pocket_topo::{parser, Color, Element, Point, Reference, Shot, ShotFlags, StationId, Trip};

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
#[test]
fn parses_elements() {
	let contents = fixture("outline.top");

	let document = parser::parse(&contents).expect("invalid document");

	let mut elements = document.outline.elements.iter().rev();
	assert_eq!(elements.len(), 24);

	let mut element: &Element;

	element = elements.next().unwrap();
	let polygon = match element {
		Element::Polygon(polygon) => polygon,
		_ => panic!(),
	};

	assert_eq!(polygon.color, Color::Black);
	assert_eq!(
		polygon.points,
		[
			(200, -9800),
			(600, -9800),
			(600, -9700),
			(800, -9700),
			(1500, -9400),
			(2200, -9200),
			(2200, -9100),
			(2500, -9000),
			(2500, -8900),
			(2700, -8900),
			(4100, -7900),
			(5600, -7200),
			(5800, -7200),
			(6700, -6900),
			(7100, -6800),
			(7400, -6800),
			(7700, -6700),
			(8000, -6600),
		]
		.into_iter()
		.map(|(x, y)| Point { x, y })
		.collect()
	);

	element = elements.next().unwrap();
	let polygon = match element {
		Element::Polygon(polygon) => polygon,
		_ => panic!(),
	};

	assert_eq!(polygon.color, Color::Gray);
	assert_eq!(
		polygon.points,
		[
			(8200, -6400),
			(8200, -4900),
			(8300, -4700),
			(8300, -4500),
			(8400, -4200),
			(9200, -1900),
			(9600, -1100),
			(9600, -900),
			(9800, -300),
			(9900, 100),
			(10000, 100),
			(10000, 300),
			(10100, 300),
			(10100, 1700),
		]
		.into_iter()
		.map(|(x, y)| Point { x, y })
		.collect()
	);

	element = elements.next().unwrap();
	let polygon = match element {
		Element::Polygon(polygon) => polygon,
		_ => panic!(),
	};

	assert_eq!(polygon.color, Color::Brown);
	assert_eq!(
		polygon.points,
		[
			(10000, 2400),
			(10000, 2500),
			(9900, 2500),
			(9500, 3600),
			(8100, 5800),
			(8000, 6100),
			(7700, 6300),
			(7700, 6500),
			(7400, 6800),
			(7400, 6900),
			(7300, 6900),
			(7300, 7000),
			(7200, 7000),
			(7100, 7200),
			(6400, 7900),
			(6300, 7900),
			(6300, 8000),
			(6100, 8000),
			(6100, 8100),
			(6000, 8100),
			(6000, 8200),
			(5700, 8300),
			(5700, 8400),
			(5600, 8400),
			(5600, 8500),
			(5300, 8600),
			(5300, 8700),
			(5000, 8700),
			(5000, 8800),
			(4800, 8800),
			(4800, 8900),
			(4600, 8900)
		]
		.into_iter()
		.map(|(x, y)| Point { x, y })
		.collect()
	);

	element = elements.next().unwrap();
	let cross_section = match element {
		Element::CrossSection(cross_section) => cross_section,
		_ => panic!(),
	};

	assert_eq!(
		cross_section.position,
		Point {
			x: -5700,
			y: -15600,
		}
	);

	assert_eq!(cross_section.station, StationId::MajorMinor(1, 0,));
	assert_eq!(cross_section.direction, 0);

	// ignore the 16 polygon elements which make up the cross-section drawing
	for _ in 0..16 {
		elements.next();
	}

	element = elements.next().unwrap();
	let polygon = match element {
		Element::Polygon(polygon) => polygon,
		_ => panic!(),
	};

	assert_eq!(polygon.color, Color::Blue);
	assert_eq!(
		polygon.points,
		[
			(4400, 9100),
			(1800, 9100),
			(800, 8900),
			(400, 8900),
			(-100, 8800),
			(-600, 8800),
			(-900, 8700),
			(-1100, 8700),
			(-1400, 8600),
			(-2600, 8600),
			(-3200, 8800),
			(-3700, 8800),
			(-3700, 8900),
			(-4300, 8900)
		]
		.into_iter()
		.map(|(x, y)| Point { x, y })
		.collect()
	);

	element = elements.next().unwrap();
	let polygon = match element {
		Element::Polygon(polygon) => polygon,
		_ => panic!(),
	};

	assert_eq!(polygon.color, Color::Red);
	assert_eq!(
		polygon.points,
		[
			(-4500, 8800),
			(-4500, 8200),
			(-4800, 7700),
			(-5100, 7000),
			(-5200, 7000),
			(-5200, 6800),
			(-5300, 6800),
			(-5400, 6500),
			(-5500, 6500),
			(-5500, 6400),
			(-5600, 6400),
			(-5600, 6300),
			(-5700, 6300),
			(-5800, 6100),
			(-6500, 5400),
			(-6600, 5200),
			(-7700, 4100),
			(-7800, 4100),
			(-7800, 4000),
			(-8000, 3900),
			(-8100, 3700),
			(-8200, 3700),
			(-8200, 3600),
			(-8300, 3600),
			(-8300, 3400),
			(-8400, 3400),
			(-8400, 3300),
			(-8500, 3300),
			(-8500, 3200),
			(-8600, 3200),
			(-8600, 3100),
			(-8700, 3100),
			(-8700, 3000),
			(-8800, 3000),
			(-8800, 2800),
			(-8900, 2800),
			(-8900, 2700),
			(-9000, 2700),
			(-9000, 2500),
			(-9100, 2500),
			(-9200, 2200),
			(-9300, 2200),
			(-9300, 2100),
			(-9500, 2100),
			(-9500, 2000)
		]
		.into_iter()
		.map(|(x, y)| Point { x, y })
		.collect()
	);

	element = elements.next().unwrap();
	let polygon = match element {
		Element::Polygon(polygon) => polygon,
		_ => panic!(),
	};

	assert_eq!(polygon.color, Color::Orange);
	assert_eq!(
		polygon.points,
		[
			(-9700, 2100),
			(-9700, 800),
			(-9600, 600),
			(-9600, 300),
			(-9500, 0),
			(-9500, -200),
			(-9300, -800),
			(-9200, -1300),
			(-8800, -2600),
			(-8700, -3200),
			(-8600, -3500),
			(-8500, -3500),
			(-8500, -3700),
			(-8400, -3700),
			(-8400, -3900),
			(-8100, -4800),
			(-8000, -5300),
			(-7800, -6100),
			(-7800, -6200)
		]
		.into_iter()
		.map(|(x, y)| Point { x, y })
		.collect()
	);

	element = elements.next().unwrap();
	let polygon = match element {
		Element::Polygon(polygon) => polygon,
		_ => panic!(),
	};

	assert_eq!(polygon.color, Color::Green);
	assert_eq!(
		polygon.points,
		[
			(-7600, -6000),
			(-7500, -6200),
			(-6600, -7000),
			(-6400, -7000),
			(-6400, -7100),
			(-6200, -7100),
			(-5900, -7200),
			(-5900, -7300),
			(-5700, -7300),
			(-5700, -7400),
			(-5400, -7500),
			(-5400, -7600),
			(-5300, -7600),
			(-5300, -7700),
			(-5100, -7700),
			(-5100, -7800),
			(-5000, -7800),
			(-5000, -7900),
			(-4700, -8000),
			(-4700, -8100),
			(-4300, -8200),
			(-4300, -8300),
			(-4000, -8300),
			(-3700, -8500),
			(-3500, -8500),
			(-2600, -8800),
			(-2600, -8900),
			(-2400, -8900),
			(-2400, -9000),
			(-2300, -9000),
			(-2300, -9100),
			(-2100, -9100),
			(-900, -9500),
			(-800, -9500),
			(-800, -9600),
			(-600, -9600),
			(-300, -9800)
		]
		.into_iter()
		.map(|(x, y)| Point { x, y })
		.collect()
	);

	assert!(elements.next().is_none());
}

#[test]
fn parses_mappings() {
	let contents = fixture("outline.top");

	let document = parser::parse(&contents).expect("invalid document");

	let mapping = document.mapping;
	assert_eq!(mapping.origin, Point { x: 0, y: 0 });
	assert_eq!(mapping.scale, 500);

	let mapping = document.outline.mapping;
	assert_eq!(mapping.origin, Point { x: 0, y: 0 });
	assert_eq!(mapping.scale, 500);

	let mapping = document.sideview.mapping;
	assert_eq!(mapping.origin, Point { x: 0, y: 0 });
	assert_eq!(mapping.scale, 500);
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
