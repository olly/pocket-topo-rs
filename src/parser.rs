use chrono::NaiveDateTime;
use nom::{
	branch::alt,
	bytes::complete::{tag, take, take_while},
	multi::{length_count, many_till},
	number::complete::{le_i16, le_i32, le_i64, le_u32, le_u8},
	Finish, IResult,
};
use thiserror::Error;

use crate::{
	Color, CrossSection, Drawing, Element, Mapping, Point, Polygon, Reference, Shot, ShotFlags,
	StationId, Trip,
};

#[derive(Debug)]
pub struct Document<'a> {
	pub references: Box<[Reference<'a>]>,
	pub shots: Box<[Shot<'a>]>,
	pub trips: Box<[Trip<'a>]>,
	pub mapping: Mapping,
	pub outline: Drawing,
	pub sideview: Drawing,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ParseError<'a> {
	#[error("invalid color: {0:#04X?}")]
	InvalidColor(u8),

	#[error("invalid header: {0:?}")]
	InvalidHeader(&'a [u8]),

	#[error("undefined station")]
	UndefinedStation,

	#[error("unknown error")]
	UnknownError,

	#[error("unsupported version: {0}")]
	UnsupportedVersion(u8),

	#[error(transparent)]
	Utf8Error(#[from] std::str::Utf8Error),
}

impl<'a, I> nom::error::ParseError<I> for ParseError<'a> {
	fn from_error_kind(_input: I, _kind: nom::error::ErrorKind) -> Self {
		Self::UnknownError
	}

	fn append(_input: I, _kind: nom::error::ErrorKind, other: Self) -> Self {
		other
	}
}

const HEADER: &[u8; 3] = b"Top";
const VERSION: u8 = 0x3;

pub fn parse(input: &[u8]) -> Result<Document, ParseError> {
	parse_internal(input).finish().map(|(_, document)| document)
}

// File = {
//   Byte 'T'
//   Byte 'o'
//   Byte 'p'
// 	 Byte 3  // version
// 	 Int32 tripCount
// 	 Trip[tripCount] trips
// 	 Int32 shotCount
// 	 Shot[shotCount] shots
// 	 Int32 refCount
// 	 Reference[refCount] references
// 	 Mapping overview
// 	 Drawing outline
// 	 Drawing sideview
// }
fn parse_internal(input: &[u8]) -> IResult<&[u8], Document, ParseError> {
	let (input, _) = parse_header(input)?;
	let (input, _) = parse_version(input)?;
	let (input, trips) = parse_trips(input)?;
	let (input, shots) = parse_shots(input)?;
	let (input, references) = parse_references(input)?;

	let (input, mapping) = parse_mapping(input)?;
	let (input, outline) = parse_drawing(input)?;
	let (input, sideview) = parse_drawing(input)?;

	Ok((
		input,
		Document {
			references,
			shots,
			trips,
			mapping,
			outline,
			sideview,
		},
	))
}

fn parse_header(input: &[u8]) -> IResult<&[u8], &[u8], ParseError> {
	tag(HEADER)(input).map_err(|_: nom::Err<ParseError>| {
		let found = input.chunks(HEADER.len()).next().unwrap_or(b"");
		nom::Err::Failure(ParseError::InvalidHeader(found))
	})
}

fn parse_version(input: &[u8]) -> IResult<&[u8], u8, ParseError> {
	let (input, version) = le_u8(input)?;

	if version != VERSION {
		return Err(nom::Err::Failure(ParseError::UnsupportedVersion(version)));
	}

	Ok((input, version))
}

// XSectionElement = {
//   Byte 3  // id
// 	 Point pos  // drawing position
// 	 Id station
// 	 Int32 direction // -1: horizontal, >=0; projection azimuth (internal angle units)
// }
fn parse_cross_section(input: &[u8]) -> IResult<&[u8], Element, ParseError> {
	let (input, _) = tag([0x3_u8])(input)?;

	let (input, position) = parse_point(input)?;
	let (input, station) = parse_station_id(input)?;
	let (input, direction) = le_i32(input)?;

	let station = match station {
		Some(station) => station,
		None => return Err(nom::Err::Error(ParseError::UndefinedStation)),
	};

	let cross_section = Element::CrossSection(CrossSection {
		position,
		station,
		direction,
	});

	Ok((input, cross_section))
}

fn parse_datetime(input: &[u8]) -> IResult<&[u8], NaiveDateTime, ParseError> {
	const NANOSECONDS: i64 = 10000000;
	const SECONDS_FROM_DOT_NET_EPOCH_TO_UNIX_EPOCH: i64 = 62135596800;

	let (input, ticks) = le_i64(input)?;

	let seconds = (ticks / NANOSECONDS) - SECONDS_FROM_DOT_NET_EPOCH_TO_UNIX_EPOCH;
	let nsecs = (ticks % NANOSECONDS) as u32;

	let time = NaiveDateTime::from_timestamp(seconds, nsecs);

	Ok((input, time))
}

// Drawing = {
//   Mapping mapping
//   Element[] elements
//   Byte 0  // end of element list
// }
fn parse_drawing(input: &[u8]) -> IResult<&[u8], Drawing, ParseError> {
	let (input, mapping) = parse_mapping(input)?;
	let (input, (elements, _)) = many_till(parse_element, tag([0x0_u8]))(input)?;

	let drawing = Drawing {
		mapping,
		elements: elements.into_boxed_slice(),
	};

	Ok((input, drawing))
}

// Element = {
//   Byte id  // element type
//   ...
// }
fn parse_element(input: &[u8]) -> IResult<&[u8], Element, ParseError> {
	alt((parse_polygon, parse_cross_section))(input)
}

// Mapping = {  // least recently used scroll position and scale
//   Point origin // middle of screen relative to first reference
// 	 Int32 scale  // 10..50000
// }
fn parse_mapping(input: &[u8]) -> IResult<&[u8], Mapping, ParseError> {
	let (input, origin) = parse_point(input)?;
	let (input, scale) = le_i32(input)?;

	let mapping = Mapping { origin, scale };

	Ok((input, mapping))
}

// Point = {  // world coordinates relative to first station in file
//   Int32 x  // mm
//   Int32 y  // mm
// }
fn parse_point(input: &[u8]) -> IResult<&[u8], Point, ParseError> {
	let (input, x) = le_i32(input)?;
	let (input, y) = le_i32(input)?;

	let point = Point { x, y };

	Ok((input, point))
}

// PolygonElement = {
//   Byte 1  // id
// 	 Int32 pointCount
// 	 Point[pointCount] points // open polygon
// 	 Byte color // black = 1, gray = 2, brown = 3, blue = 4; red = 5, green = 6, orange = 7
// }
fn parse_polygon(input: &[u8]) -> IResult<&[u8], Element, ParseError> {
	let (input, _) = tag([0x1_u8])(input)?;

	let (input, points) = length_count(le_u32, parse_point)(input)?;
	let (input, color) = le_u8(input)?;

	let color = match color {
		0x1_u8 => Color::Black,
		0x2_u8 => Color::Gray,
		0x3_u8 => Color::Brown,
		0x4_u8 => Color::Blue,
		0x5_u8 => Color::Red,
		0x6_u8 => Color::Green,
		0x7_u8 => Color::Orange,
		invalid => return Err(nom::Err::Error(ParseError::InvalidColor(invalid))),
	};

	let polygon = Element::Polygon(Polygon {
		points: points.into_boxed_slice(),
		color,
	});

	Ok((input, polygon))
}

fn parse_shots(input: &[u8]) -> IResult<&[u8], Box<[Shot]>, ParseError> {
	length_count(le_u32, parse_shot)(input)
		.map(|(input, collection)| (input, collection.into_boxed_slice()))
}

// Shot = {
//   Id from
// 	 Id to
// 	 Int32 dist  // mm
// 	 Int16 azimuth  // internal angle units (full circle = 2^16, north = 0, east = 0x4000)
// 	 Int16 inclination // internal angle units (full circle = 2^16, up = 0x4000, down = 0xC000)
// 	 Byte flags  // bit0: flipped shot
// 	 Byte roll   // roll angle (full circle = 256, disply up = 0, left = 64, down = 128)
// 	 Int16 tripIndex  // -1: no trip, >=0: trip reference
// 	 if (flags & 2)
// 	   String comment
// }
fn parse_shot(input: &[u8]) -> IResult<&[u8], Shot, ParseError> {
	let (input, from) = parse_station_id(input)?;
	let (input, to) = parse_station_id(input)?;
	let (input, distance) = le_i32(input)?;
	let (input, azimuth) = le_i16(input)?;
	let (input, inclination) = le_i16(input)?;
	let (input, flags) = le_u8(input)?;
	let (input, roll) = le_u8(input)?;
	let (input, trip_index) = le_i16(input)?;

	let flags = ShotFlags { bits: flags };

	let (input, comment) = if flags.contains(ShotFlags::HAS_COMMENT) {
		let (input, string) = parse_string(input)?;
		(input, Some(string))
	} else {
		(input, None)
	};

	let shot = Shot {
		from,
		to,
		distance,
		azimuth,
		inclination,
		flags,
		roll,
		trip_index,
		comment,
	};

	Ok((input, shot))
}

// Id = { // station identification
//   Int32 value  // 0x80000000: undefined, <0: plain numbers + 0x80000001, >=0: major<<16|minor
// }
fn parse_station_id(input: &[u8]) -> IResult<&[u8], Option<StationId>, ParseError> {
	const UNDEFINED: u32 = 0b10000000000000000000000000000000;

	let (input, station_id) = le_u32(input)?;

	let station_id = match station_id {
		UNDEFINED => None,
		x if x & UNDEFINED == UNDEFINED => {
			let x = (x ^ UNDEFINED) - 1;
			Some(StationId::Plain(x))
		}
		x => {
			let major: u16 = (x >> 16) as u16;
			let minor: u16 = (x) as u16;
			Some(StationId::MajorMinor(major, minor))
		}
	};

	Ok((input, station_id))
}

// String = { // .Net string format
//   Byte[] length // unsigned, encoded in 7 bit chunks, little endian, bit7 set in all but the last byte
//   Byte[length]  // UTF8 encoded, 1 to 3 bytes per character, not 0 terminated
// }
fn parse_string(input: &[u8]) -> IResult<&[u8], &str, ParseError> {
	let (input, length) = parse_variable_length_little_endian_int(input)?;
	let (input, bytes) = take(length)(input)?;

	let str = match std::str::from_utf8(bytes) {
		Ok(str) => str,
		Err(err) => return Err(nom::Err::Error(ParseError::from(err))),
	};

	Ok((input, str))
}

fn parse_references(input: &[u8]) -> IResult<&[u8], Box<[Reference]>, ParseError> {
	length_count(le_u32, parse_reference)(input)
		.map(|(input, collection)| (input, collection.into_boxed_slice()))
}

// Reference = {
//   Id station
// 	 Int64 east     // mm
// 	 Int64 north    // mm
// 	 Int32 altitude // mm above sea level
// 	 String comment
// }
fn parse_reference(input: &[u8]) -> IResult<&[u8], Reference, ParseError> {
	let (input, station) = parse_station_id(input)?;
	let (input, east) = le_i64(input)?;
	let (input, north) = le_i64(input)?;
	let (input, altitude) = le_i32(input)?;
	let (input, comment) = parse_string(input)?;

	let reference = Reference {
		station,
		east,
		north,
		altitude,
		comment,
	};

	Ok((input, reference))
}

fn parse_trips(input: &[u8]) -> IResult<&[u8], Box<[Trip]>, ParseError> {
	length_count(le_u32, parse_trip)(input)
		.map(|(input, collection)| (input, collection.into_boxed_slice()))
}

// Trip = {
//   Int64 time  // ticks (100ns units starting at 1.1.1)
// 	 String comment
// 	 Int16 declination  // internal angle units (full circle = 2^16)
// }
fn parse_trip(input: &[u8]) -> IResult<&[u8], Trip, ParseError> {
	let (input, time) = parse_datetime(input)?;
	let (input, comment) = parse_string(input)?;
	let (input, declination) = le_i16(input)?;

	let trip = Trip {
		time,
		comment,
		declination,
	};

	Ok((input, trip))
}

// unsigned, encoded in 7 bit chunks, little endian, bit7 set in all but the last byte
fn parse_variable_length_little_endian_int(input: &[u8]) -> IResult<&[u8], usize, ParseError> {
	const BIT_7_SET: u8 = 0b10000000;

	let (input, bytes) = take_while(|byte| byte & BIT_7_SET == BIT_7_SET)(input)?;
	let (input, byte) = take(1_u8)(input)?;

	let mut result: usize = 0;
	for (i, v) in bytes.iter().chain(byte.iter()).enumerate() {
		// Convert to usize value to ensure we lose information when we shift left
		let b = *v as usize;

		// Mask to remove the continuation bit
		let b = b & 0b01111111;

		// Shift left 7 bytes for each byte of input
		let b = b << (7 * i);

		// OR exisitng result and b together
		result |= b;
	}

	Ok((input, result))
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_parse_header() {
		assert!(parse_header(b"Top").is_ok());
		assert!(parse_header(b"To").is_err());
		assert!(parse_header(b"TOP").is_err());
	}

	#[test]
	fn test_invalid_header() {
		let contents = vec![b'T', b'O', b'P', 0x3];
		let result = parse(&contents);

		let error = result.expect_err("expected `ParserError`");
		assert_eq!(error, ParseError::InvalidHeader(&[b'T', b'O', b'P']));

		assert_eq!(error.to_string(), "invalid header: [84, 79, 80]");
	}

	#[test]
	fn test_invalid_version() {
		let contents = vec![b'T', b'o', b'p', 0x2];
		let result = parse(&contents);

		let error = result.expect_err("expected `ParserError`");
		assert_eq!(error, ParseError::UnsupportedVersion(0x2));

		assert_eq!(error.to_string(), "unsupported version: 2");
	}

	#[test]
	fn test_parse_station_id() {
		let (_, station_id) = parse_station_id(&[0x00, 0x00, 0x00, 0x80]).unwrap();
		assert_eq!(station_id, None);

		let (_, station_id) = parse_station_id(&[0x00, 0x00, 0x01, 0x00]).unwrap();
		assert_eq!(station_id, Some(StationId::MajorMinor(1, 0)));

		let (_, station_id) = parse_station_id(&[0x01, 0x00, 0x2A, 0x00]).unwrap();
		assert_eq!(station_id, Some(StationId::MajorMinor(42, 1)));

		let (_, station_id) = parse_station_id(&[0x01, 0x00, 0x00, 0x40]).unwrap();
		assert_eq!(station_id, Some(StationId::MajorMinor(16384, 1)));

		let (_, station_id) = parse_station_id(&[0x00, 0x40, 0x0, 0x40]).unwrap();
		assert_eq!(station_id, Some(StationId::MajorMinor(16384, 16384)));

		let (_, station_id) = parse_station_id(&[0x00, 0x00, 0xFF, 0x7F]).unwrap();
		assert_eq!(station_id, Some(StationId::MajorMinor(32767, 0)));

		let (_, station_id) = parse_station_id(&[0xFF, 0xFF, 0xFF, 0x7F]).unwrap();
		assert_eq!(station_id, Some(StationId::MajorMinor(32767, 65535)));

		let (_, station_id) = parse_station_id(&[0x01, 0x00, 0x00, 0x80]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(0)));

		let (_, station_id) = parse_station_id(&[0x02, 0x00, 0x00, 0x80]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(1)));

		let (_, station_id) = parse_station_id(&[0x03, 0x00, 0x00, 0x80]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(2)));

		let (_, station_id) = parse_station_id(&[0x04, 0x00, 0x00, 0x80]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(3)));

		let (_, station_id) = parse_station_id(&[0x05, 0x00, 0x00, 0x80]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(4)));

		let (_, station_id) = parse_station_id(&[0x42, 0x42, 0x0f, 0x80]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(1000001)));

		let (_, station_id) = parse_station_id(&[0x00, 0xFF, 0x0F, 0x80]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(1048319)));

		let (_, station_id) = parse_station_id(&[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
		assert_eq!(station_id, Some(StationId::Plain(2147483646)));
	}

	#[test]
	fn test_parse_variable_length_little_endian_int() {
		let (_, result) = parse_variable_length_little_endian_int(&[0x00_u8]).unwrap();
		assert_eq!(result, 0x0_usize);

		let (input, result) = parse_variable_length_little_endian_int(&[0x00_u8, 0x00_u8]).unwrap();
		assert_eq!(input, &[0x00_u8]);
		assert_eq!(result, 0x0_usize);

		let (_, result) = parse_variable_length_little_endian_int(&[0x2b_u8]).unwrap();
		assert_eq!(result, 43_usize);

		let (_, result) = parse_variable_length_little_endian_int(&[0b00000000_u8]).unwrap();
		assert_eq!(result, 0_usize);

		let (_, result) = parse_variable_length_little_endian_int(&[0b00000001_u8]).unwrap();
		assert_eq!(result, 1_usize);

		let (_, result) = parse_variable_length_little_endian_int(&[0b00000010_u8]).unwrap();
		assert_eq!(result, 2_usize);

		let (_, result) =
			parse_variable_length_little_endian_int(&[0b11111111_u8, 0b00000001]).unwrap();
		assert_eq!(result, 255_usize);

		let (_, result) =
			parse_variable_length_little_endian_int(&[0b10000000_u8, 0b00000000_u8]).unwrap();
		assert_eq!(result, 0x0_usize);
	}
}
