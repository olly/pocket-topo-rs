pub mod parser;

use bitflags::bitflags;

#[derive(Debug)]
pub struct Shot {
	pub from: Option<StationId>,
	pub to: Option<StationId>,
	pub azimuth: i16,
	pub distance: i32,
	pub inclination: i16,
	pub flags: ShotFlags,
	pub roll: u8,
	pub trip_index: i16,
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
