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

pub trait ZisrawInterface{
	fn get_metadata(&self,file:&mut BufReader<File>) -> Result<zisraw_structs::Metadata>;
	fn get_directory(&self,file:&mut BufReader<File>) -> Result<zisraw_structs::Directory>;
	fn get_metadata_element(&self,file:&mut BufReader<File>) -> Result<xmltree::Element>{
		let mut cache = self.get_metadata(file)?.cache;
		Ok(cache.get().clone())
	}
	fn get_metadata_xml(&self,file:&mut BufReader<File>) -> Result<String>{
		let e = self.get_metadata(file)?;
		Ok(e.cache.source.clone())
	}
	fn get_pyramid(&self,file:&mut BufReader<File>)-> Result<()>{
		let entries=self.get_directory(file)?.Entries;
		for e in entries{
			if e.PyramidType == 0 { //not a pyramid actually

			} else {
				let scale = e.DimensionEntries[0].Size as f32 / e.DimensionEntries[0].StoredSize as f32;
			}
		}
		Ok(())
	}
}
