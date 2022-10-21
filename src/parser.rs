use nom::{
	bytes::complete::tag,
	multi::count,
	number::complete::{le_i16, le_i32, le_i64, le_u16, le_u32, le_u8},
	IResult,
};
use thiserror::Error;

#[derive(Debug)]
pub struct Document {
	references: Box<[()]>,
	shots: Box<[()]>,
	trips: Box<[()]>,
}

impl Document {
	pub fn references(&self) -> &[()] {
		&self.references
	}

	pub fn shots(&self) -> &[()] {
		&self.shots
	}

	pub fn trips(&self) -> &[()] {
		&self.trips
	}
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

	Ok((
		input,
		Document {
			references: references.into_boxed_slice(),
			shots: shots.into_boxed_slice(),
			trips: trips.into_boxed_slice(),
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

fn parse_shots(input: &[u8]) -> IResult<&[u8], Vec<()>> {
	// length_count(le_u32, parse_shot)(input)

	let length: IResult<&[u8], u32> = le_u32(input);
	let (input, length): (_, u32) = length.unwrap_or((input, 0));
	count(parse_shot, length as usize)(input)
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
fn parse_shot(input: &[u8]) -> IResult<&[u8], ()> {
	let (input, _from) = le_i32(input)?;
	let (input, _to) = le_i32(input)?;
	let (input, _distance) = le_i32(input)?;
	let (input, _azimuth) = le_i16(input)?;
	let (input, _inclination) = le_i16(input)?;
	let (input, _flags) = le_u8(input)?;
	let (input, _roll) = le_u8(input)?;
	let (input, _trip_index) = le_u16(input)?;

	// TODO: parse conditional comment

	Ok((input, ()))
}

// String = { // .Net string format
//   Byte[] length // unsigned, encoded in 7 bit chunks, little endian, bit7 set in all but the last byte
//   Byte[length]  // UTF8 encoded, 1 to 3 bytes per character, not 0 terminated
// }
fn parse_string(input: &[u8]) -> IResult<&[u8], ()> {
	// TODO: implement
	let (input, length) = le_u8(input)?;
	debug_assert_eq!(length, 0);

	Ok((input, ()))
}

fn parse_references(input: &[u8]) -> IResult<&[u8], Vec<()>> {
	// length_count(le_u32, parse_reference)(input)

	let length: IResult<&[u8], u32> = le_u32(input);
	let (input, length): (_, u32) = length.unwrap_or((input, 0));
	count(parse_reference, length as usize)(input)
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

fn parse_trips(input: &[u8]) -> IResult<&[u8], Vec<()>> {
	// length_count(le_u32, parse_trip)(input)

	let length: IResult<&[u8], u32> = le_u32(input);
	let (input, length): (_, u32) = length.unwrap_or((input, 0));
	count(parse_trip, length as usize)(input)
}

// Trip = {
//   Int64 time  // ticks (100ns units starting at 1.1.1)
// 	 String comment
// 	 Int16 declination  // internal angle units (full circle = 2^16)
// }
fn parse_trip(input: &[u8]) -> IResult<&[u8], ()> {
	let (input, _time) = le_i64(input)?;
	let (input, _comment) = parse_string(input)?;
	let (input, _declination) = le_i16(input)?;

	Ok((input, ()))
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
}
