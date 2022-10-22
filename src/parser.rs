use nom::{
	bytes::complete::{tag, take, take_while},
	multi::length_count,
	number::complete::{le_i16, le_i32, le_i64, le_u32, le_u8},
	IResult,
};
use thiserror::Error;

use crate::{Shot, ShotFlags, StationId, Trip};

#[derive(Debug)]
pub struct Document<'a> {
	pub references: Box<[()]>,
	pub shots: Box<[Shot<'a>]>,
	pub trips: Box<[Trip<'a>]>,
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ParserError<'a> {
	#[error("invalid header: {0:?}")]
	InvalidHeader(&'a [u8]),

	#[error("unsupported version: {0}")]
	UnsupportedVersion(u8),

	#[error("unknown error")]
	UnknownError,
}

const HEADER: &[u8; 3] = b"Top";
const VERSION: u8 = 0x3;

pub fn parse(input: &[u8]) -> Result<Document, ParserError> {
	let (input, _) = parse_header(input)?;
	let (input, _) = parse_version(input)?;

	// TODO: remove unwrap
	Ok(parse_internal(input).unwrap().1)
}

fn parse_internal(input: &[u8]) -> IResult<&[u8], Document> {
	let (input, trips) = parse_trips(input)?;
	let (input, shots) = parse_shots(input)?;
	let (input, references) = parse_references(input)?;

	let (input, _mapping) = parse_mapping(input)?;
	let (input, _outline) = parse_drawing(input)?;
	let (input, _sideview) = parse_drawing(input)?;

	Ok((
		input,
		Document {
			references,
			shots,
			trips,
		},
	))
}

fn parse_header(input: &[u8]) -> Result<(&[u8], &[u8]), ParserError> {
	let result: IResult<&[u8], &[u8]> = tag(HEADER)(input);

	result.map_err(|_| {
		let found = input.chunks(HEADER.len()).next().unwrap_or(b"");
		ParserError::InvalidHeader(found)
	})
}

fn parse_version(input: &[u8]) -> Result<(&[u8], u8), ParserError> {
	let (input, version) =
		le_u8::<&[u8], nom::error::Error<&[u8]>>(input).map_err(|_| ParserError::UnknownError)?;

	if version != VERSION {
		return Err(ParserError::UnsupportedVersion(version));
	}

	Ok((input, version))
}

// Drawing = {
//   Mapping mapping
//   Element[] elements
//   Byte 0  // end of element list
// }
fn parse_drawing(input: &[u8]) -> IResult<&[u8], ()> {
	let (input, _mapping) = parse_mapping(input)?;
	let (input, _terminator) = tag(&[0x0])(input)?;

	Ok((input, ()))
}

// Mapping = {  // least recently used scroll position and scale
//   Point origin // middle of screen relative to first reference
// 	 Int32 scale  // 10..50000
// }
fn parse_mapping(input: &[u8]) -> IResult<&[u8], ()> {
	let (input, _origin) = parse_point(input)?;
	let (input, _scale) = le_i32(input)?;

	Ok((input, ()))
}

// Point = {  // world coordinates relative to first station in file
//   Int32 x  // mm
//   Int32 y  // mm
// }
fn parse_point(input: &[u8]) -> IResult<&[u8], ()> {
	let (input, _x) = le_i32(input)?;
	let (input, _y) = le_i32(input)?;

	Ok((input, ()))
}

fn parse_shots(input: &[u8]) -> IResult<&[u8], Box<[Shot]>> {
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
fn parse_shot(input: &[u8]) -> IResult<&[u8], Shot> {
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
fn parse_station_id(input: &[u8]) -> IResult<&[u8], Option<StationId>> {
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
fn parse_string(input: &[u8]) -> IResult<&[u8], &str> {
	let (input, length) = parse_variable_length_little_endian_int(input)?;
	let (input, string) = take(length)(input)?;

	// TODO:: remove unwrap
	Ok((input, std::str::from_utf8(string).unwrap()))
}

fn parse_references(input: &[u8]) -> IResult<&[u8], Box<[()]>> {
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
fn parse_reference(input: &[u8]) -> IResult<&[u8], ()> {
	let (input, _station) = le_i32(input)?;
	let (input, _east) = le_i64(input)?;
	let (input, _north) = le_i64(input)?;
	let (input, _altitude) = le_i32(input)?;
	let (input, _comment) = parse_string(input)?;

	Ok((input, ()))
}

fn parse_trips(input: &[u8]) -> IResult<&[u8], Box<[Trip]>> {
	length_count(le_u32, parse_trip)(input)
		.map(|(input, collection)| (input, collection.into_boxed_slice()))
}

// Trip = {
//   Int64 time  // ticks (100ns units starting at 1.1.1)
// 	 String comment
// 	 Int16 declination  // internal angle units (full circle = 2^16)
// }
fn parse_trip(input: &[u8]) -> IResult<&[u8], Trip> {
	let (input, time) = le_i64(input)?;
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
fn parse_variable_length_little_endian_int(input: &[u8]) -> IResult<&[u8], usize> {
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
		assert_eq!(error, ParserError::InvalidHeader(&[b'T', b'O', b'P']));

		assert_eq!(error.to_string(), "invalid header: [84, 79, 80]");
	}

	#[test]
	fn test_invalid_version() {
		let contents = vec![b'T', b'o', b'p', 0x2];
		let result = parse(&contents);

		let error = result.expect_err("expected `ParserError`");
		assert_eq!(error, ParserError::UnsupportedVersion(0x2));

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
