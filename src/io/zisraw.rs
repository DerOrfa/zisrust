#![allow(non_snake_case)]

use std::fs::File;
use std::io::{BufReader, Seek};
use std::io::{Error, ErrorKind, Result, SeekFrom};
use crate::io::{Endian, FileGet};

mod zisraw_impl;
mod zisraw_structs;

pub fn get_file_header(file:&mut BufReader<File>) -> Result<zisraw_structs::FileHeader>{
	file.seek(SeekFrom::Start(0))?;
	let s:zisraw_structs::Segment = file.get(&Endian::Little)?;
	match s.block {
		zisraw_structs::SegmentBlock::FileHeader(hd) => Ok(hd),
		_ => Err(Error::new(ErrorKind::InvalidInput,"Unexpected block when looking for header"))
	}
}
