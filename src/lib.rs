pub mod parser;

use bitflags::bitflags;
use chrono::NaiveDateTime;

#[derive(Debug, Eq, PartialEq)]
pub enum Color {
	Black,
	Blue,
	Brown,
	Gray,
	Green,
	Orange,
	Red,
}

#[derive(Debug)]
pub struct CrossSection {
	pub position: Point,
	pub station: StationId,
	pub direction: i32,
}

#[derive(Debug)]
pub struct Drawing {
	pub mapping: Mapping,
	pub elements: Box<[Element]>,
}

#[derive(Debug)]
pub enum Element {
	Polygon(Polygon),
	CrossSection(CrossSection),
}

#[derive(Debug)]
pub struct Mapping {
	pub origin: Point,
	pub scale: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Point {
	pub x: i32,
	pub y: i32,
}

#[derive(Debug)]
pub struct Polygon {
	pub points: Box<[Point]>,
	pub color: Color,
}

#[derive(Debug)]
pub struct Reference<'a> {
	pub station: Option<StationId>,
	pub east: i64,     // mm
	pub north: i64,    // mm
	pub altitude: i32, // mm above sea level
	pub comment: &'a str,
}

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
