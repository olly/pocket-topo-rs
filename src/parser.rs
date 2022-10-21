use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
pub enum ParserError<'a> {
	#[error("invalid header: {0:?}")]
	InvalidHeader(&'a [u8]),
}

static HEADER: [u8; 3] = [b'T', b'o', b'p'];

pub fn parse<'a>(contents: &'a [u8]) -> Result<(), ParserError<'a>> {
	let header = &contents[0..3];
	if header == HEADER {
		Ok(())
	} else {
		return Err(ParserError::InvalidHeader(header));
	}
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
}
