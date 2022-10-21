use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ParserError<'a> {
	#[error("invalid header: {0:?}")]
	InvalidHeader(&'a [u8]),

	#[error("unsupported version: {0}")]
	UnsupportedVersion(u8),
}

static HEADER: [u8; 3] = [b'T', b'o', b'p'];
static VERSION: u8 = 0x3;

pub fn parse<'a>(contents: &'a [u8]) -> Result<(), ParserError<'a>> {
	let header = &contents[0..3];
	if header != HEADER {
		return Err(ParserError::InvalidHeader(header));
	}

	let version = contents[3];
	if version != VERSION {
		return Err(ParserError::UnsupportedVersion(version));
	}

	Ok(())
}

#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn test_valid_header() {
		let contents = vec![b'T', b'o', b'p', 0x3];
		let result = parse(&contents);
		assert!(result.is_ok());
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
