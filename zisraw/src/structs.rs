#![allow(non_snake_case)]

use uuid::Uuid;
use iobase::DataFromFile;
use xmltree;
use crate::{Error, Result, Error::Own};
use super::ZisrawInterface;
use std::os::unix::fs::FileExt;
use std::sync::Arc;
use super::segment::{Segment,SegmentBlock};


#[derive(Debug)]
pub struct FileHeader{
	pub version:[u32;2],
	pub PrimaryFileGuid:Uuid,
	pub FileGuid:Uuid,
	pub FilePart:i32,
	pub DirectoryPosition:u64,
	pub MetadataPosition:u64,
	pub UpdatePending:bool,
	pub AttachmentDirectoryPosition:u64
}

#[derive(Debug)]
pub struct Directory{
	pub Entries:Vec<DirectoryEntryDV>
}

#[derive(Debug)]
pub struct Metadata{
	//pub AttachmentSize:i32, //NOT USED CURRENTLY
	pub cache:iobase::basic::Cached<String,xmltree::Element>
}

#[derive(Debug)]
pub struct AttachmentDirectory{
	pub Entries:Vec<AttachmentEntryA1>
}

#[derive(Debug)]
pub struct Attachment{
	pub Entry:AttachmentEntryA1,
	pub Data:DataFromFile
}

#[derive(Debug)]
pub struct AttachmentEntryA1{
	pub SchemaType:String, //4 bytes
	pub FilePosition:u64,
	pub FilePart:i32,
	pub ContentGuid:Uuid,
	pub ContentFileType:String, //8 bytes
	pub Name:String //80 bytes
}

#[derive(Debug)]
pub struct SubBlock{
	pub Entry:DirectoryEntryDV,
	pub Metadata: iobase::basic::Cached<String,xmltree::Element>,
	pub Data:DataFromFile,
	pub Attachment:Option<DataFromFile>
}

#[derive(Debug)]
pub struct DirectoryEntryDV{
	pub SchemaType:String,//4 bytes
	pub PixelType:i32,
	pub FilePosition:u64,
	pub FilePart:i32,
	pub Compression:i32,
	pub PyramidType:u8,
	pub dimension_map:std::collections::HashMap<String,DimensionEntryDV1>,
}

#[derive(Debug)]
pub struct DimensionEntryDV1{
	pub Dimension:String,//read as [char;4]
	pub Start:i32,
	pub Size:u32,
	pub StartCoordinate:f32,
	pub StoredSize:u32
}

// impl pyramid::Tile for DirectoryEntryDV{
// 	fn frame(&self) -> Rect<i32, pyramid::PixelSpace> {
// 		euclid::rect(
// 			self.dimension_map["X"].Start,
// 			self.dimension_map["Y"].Start,
// 			self.dimension_map["X"].Size as i32,
// 			self.dimension_map["Y"].Size as i32
// 		)
// 	}
//
// 	fn pixel(&self) -> pyramid::Pixel {
// 		todo!()
// 	}
//
// 	fn level(&self, scaling: i32) -> usize {
// 		if self.PyramidType > 0{
// 			assert!(scaling >1);
// 			let scale= self.dimension_map["X"].Size / self.dimension_map["X"].StoredSize;
// 			// todo Use feature(int_log)
// 			((scale as f32).log10() / (scaling as f32).log10()) as usize
// 		} else {
// 			0
// 		}
// 	}
//
// 	fn ordering_id(&self) -> i32 {
// 		self.dimension_map["M"].Start
// 	}
// }

// impl Directory {
// 	pub fn take_tiles(&mut self, scene: i32) -> Vec<Box<dyn pyramid::Tile>>{
// 		let mut ret:Vec<Box<dyn pyramid::Tile>>=Vec::with_capacity(self.Entries.len());
// 		let mut i = 0;
// 		while i < self.Entries.len() {
// 			if self.Entries[i].dimension_map["S"].Start == scene {
// 				ret.push(Box::new(self.Entries.remove(i)));
// 			} else {
// 				i += 1;
// 			}
// 		}
// 		// todo Use drain_filter once available
// 		// for t in self.Entries.drain_filter(|ed|ed.dimension_map["S"].Start == scene){
// 		// 	ret.push(Box::new(t));
// 		// }
// 		ret
// 	}
// }

impl ZisrawInterface for FileHeader{
	fn get_metadata(&self,file:&Arc<dyn FileExt>) -> Result<Metadata>{
		let s = Segment::new(file, self.MetadataPosition)?;
		if let SegmentBlock::Metadata(d) = s.block {
			Ok(d)
		} else {
			Err(Error::from("Unexpected block when looking for metadata"))
		}
	}
	fn get_directory(&self,file:&Arc<dyn FileExt>) -> Result<Directory>{
		let s:Segment = Segment::new(file, self.DirectoryPosition)?;
		if let SegmentBlock::Directory(d) = s.block {
			Ok(d)
		} else {
			Err(Error::from("Unexpected block when looking for directory"))
		}
	}
	fn get_attachments(&self,file:&Arc<dyn FileExt>)-> Result<Vec<AttachmentEntryA1>>{
		let s:Segment = Segment::new(file, self.AttachmentDirectoryPosition)?;
		let attachments= match s.block {
			SegmentBlock::AttachmentDirectory(d) => Ok(d),
			_ => Err(Error::from("Unexpected block when looking for attachments"))
		}?;
		Ok(attachments.Entries)
	}
}

impl Metadata {
	pub fn as_tree(&mut self) -> Result<xmltree::Element> {
		match self.cache.get(){
			Ok(elm) => elm // if the producer produced the data
				.get_child("Metadata").cloned()// get the child, maybe
				.ok_or(Error::from("\"Metadata\" missing in xml stream")), //if not return error
			Err(e) => Err(Own(format!("error when parsing xml data ({})",e)))
		}
	}
}

