pub mod parser;

use bitflags::bitflags;
use chrono::NaiveDateTime;

#[derive(Debug)]
pub struct Shot<'a> {
	pub from: Option<StationId>,
	pub to: Option<StationId>,
	pub azimuth: i16,
	pub distance: i32,
	pub inclination: i16,
	pub flags: ShotFlags,
	pub roll: u8,
	pub trip_index: i16,
	pub comment: Option<&'a str>,
}

bitflags! {
	pub struct ShotFlags: u8 {
		const FLIPPED = (1 << 0);
		const HAS_COMMENT = (1 << 1);
	}
}

#[derive(Debug, Eq, PartialEq)]
pub enum StationId {
	MajorMinor(u16, u16),
	Plain(u32),
}

#[derive(Debug)]
pub struct Trip<'a> {
	pub time: NaiveDateTime,
	pub comment: &'a str,
	pub declination: i16,
}
