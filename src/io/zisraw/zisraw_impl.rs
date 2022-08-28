#![allow(non_snake_case)]
use crate::io::{FileRead, FileGet, Endian, DataFromFile, basic::Cached};
use crate::io::zisraw::zisraw_structs::*;
use crate::{Error, Result};
use std::io::{Read, Seek, SeekFrom};
use euclid::Rect;
use xmltree::Element;
use crate::pyramid;
use super::ZisrawInterface;
use std::os::unix::fs::FileExt;
use std::sync::Arc;

pub fn parse_xml(source:&String) ->Element{
	Element::parse(source.as_bytes()).unwrap()
}

impl<T: Read> FileRead<T> for FileHeader{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(FileHeader{
			version: [file.get_scalar(endianess)?,file.get_scalar(endianess)?],
			Reserved1: file.get_scalar(endianess)?,
			Reserved2: file.get_scalar(endianess)?,
			PrimaryFileGuid: file.get_scalar(endianess)?,
			FileGuid: file.get_scalar(endianess)?,
			FilePart: file.get_scalar(endianess)?,
			DirectoryPosition: file.get_scalar(endianess)?,
			MetadataPosition: file.get_scalar(endianess)?,
			UpdatePending: i32::read(file,endianess)?!=0,
			AttachmentDirectoryPosition: file.get_scalar(endianess)?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for AttachmentDirectory{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let count:u32 = file.get_scalar(endianess)?;
		file.seek(SeekFrom::Start(32+256)).unwrap();
		Ok(AttachmentDirectory{
			Entries: file.get_vec(count as usize,endianess)?
		})
	}
}

impl<T: Read> FileRead<T> for AttachmentEntryA1{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(AttachmentEntryA1{
			SchemaType: file.get_ascii::<2>()?,
			Reserved: file.get_scalar(endianess)?,
			FilePosition: file.get_scalar(endianess)?,
			FilePart: file.get_scalar(endianess)?,
			ContentGuid: file.get_scalar(endianess)?,
			ContentFileType: file.get_ascii::<8>()?,
			Name: file.get_ascii::<80>()?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for Metadata{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let xml_size:i32= file.get_scalar(endianess)?;
		file.seek(SeekFrom::Start(32+256)).unwrap();//actually there is also 4 bytes reserved for AttachmentSize, but that's "NOT USED CURRENTLY"
		let xml_string=file.get_utf8(xml_size as u64)?;
		Ok(Metadata{
			cache: Cached::new(xml_string, parse_xml)
		})
	}
}

impl SubBlock{
	pub fn new<F:Read+Seek>(buffer: &mut F, file:&Arc<dyn FileExt>, offset:u64) -> Result<Self> {
		let endianess = &Endian::Little;
		let metadata_size:u32 = buffer.get_scalar(endianess)?;
		let attachment_size:u32= buffer.get_scalar(endianess)?;
		let data_size:u64 = buffer.get_scalar(endianess)?;
		let Entry = buffer.get_scalar(endianess)?;

		if buffer.stream_position().unwrap() < 32+256{
			buffer.seek(SeekFrom::Start(32+256)).unwrap();
		}
		let metadata_xml = buffer.get_utf8(metadata_size as u64)?;
		let Metadata = Cached::new(metadata_xml, parse_xml);

		let data_pos = offset+buffer.stream_position().unwrap();
		let Data = DataFromFile::new(file, data_pos, data_size as usize);

		let Attachment:Option<DataFromFile> =
			if attachment_size>0 {
				Some(DataFromFile::new(file, data_pos+data_size, attachment_size as usize))
			} else {
				None
			};
		Ok(SubBlock{Entry,Metadata,	Data, Attachment})
	}
}

impl<T: Read> FileRead<T> for DirectoryEntryDV{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let SchemaType= file.get_ascii::<2>()?;
		let PixelType = file.get_scalar(endianess)?;
		let FilePosition = file.get_scalar(endianess)?;
		let FilePart = file.get_scalar(endianess)?;
		let Compression = file.get_scalar(endianess)?;
		let buffer:[u8;6] = file.get_scalar(endianess)?;//PyramidType, and 5 reserved bytes
		let dimension_count:u32 = file.get_scalar(endianess)?;
		let DimensionEntries:Vec<DimensionEntryDV1> = file.get_vec(dimension_count as usize,endianess)?;
		let map = DimensionEntries.into_iter().map(|de|(de.Dimension.clone(),de)).collect();
		Ok(DirectoryEntryDV{
			SchemaType,	PixelType, FilePosition, FilePart, Compression,
			PyramidType:buffer[0],
			dimension_map:map
		})
	}
}

impl pyramid::Tile for DirectoryEntryDV{
	fn frame(&self) -> Rect<i32, pyramid::PixelSpace> {
		euclid::rect(
			self.dimension_map["X"].Start,
			self.dimension_map["Y"].Start,
			self.dimension_map["X"].Size as i32,
			self.dimension_map["Y"].Size as i32
		)
	}

	fn pixel(&self) -> pyramid::Pixel {
		todo!()
	}

	fn level(&self, scaling: i32) -> usize {
		if self.PyramidType > 0{
			assert!(scaling >1);
			let scale= self.dimension_map["X"].Size / self.dimension_map["X"].StoredSize;
			// todo Use feature(int_log)
			((scale as f32).log10() / (scaling as f32).log10()) as usize
		} else {
			0
		}
	}

	fn ordering_id(&self) -> i32 {
		self.dimension_map["M"].Start
	}
}

impl<T: Read> FileRead<T> for DimensionEntryDV1{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(DimensionEntryDV1{
			Dimension: file.get_ascii::<4>()?,
			Start: file.get_scalar(endianess)?,
			Size: file.get_scalar(endianess)?,
			StartCoordinate: file.get_scalar(endianess)?,
			StoredSize: file.get_scalar(endianess)?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for Directory{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let EntryCount:i32 = file.get_scalar(endianess)?;
		file.seek(SeekFrom::Start(32+128)).unwrap();
		Ok(Directory{
			Entries: file.get_vec(EntryCount as usize,endianess)?
		})
	}
}

impl Directory {
	pub fn take_tiles(&mut self, scene: i32) -> Vec<Box<dyn pyramid::Tile>>{
		let mut ret:Vec<Box<dyn pyramid::Tile>>=Vec::with_capacity(self.Entries.len());
		let mut i = 0;
		while i < self.Entries.len() {
			if self.Entries[i].dimension_map["S"].Start == scene {
				ret.push(Box::new(self.Entries.remove(i)));
			} else {
				i += 1;
			}
		}
		// todo Use drain_filter once available
		// for t in self.Entries.drain_filter(|ed|ed.dimension_map["S"].Start == scene){
		// 	ret.push(Box::new(t));
		// }
		ret
	}
}

impl Attachment{
	pub fn new<F:Read+Seek>(buffer: &mut F, file:&Arc<dyn FileExt>, offset:u64) -> Result<Self> {
		let endianess= &Endian::Little;
		let data_size:u32 = buffer.get_scalar(endianess)?;
		buffer.seek(SeekFrom::Start(32+16)).unwrap();
		let Entry = buffer.get_scalar(endianess)?;
		let Data = DataFromFile::new(file, offset+32+256 as u64, data_size as usize);
		Ok(Attachment{Entry,Data})
	}
}

impl ZisrawInterface for FileHeader{
	fn get_metadata(&self,file:&Arc<dyn FileExt>) -> Result<Metadata>{
		let s = Segment::new(file, self.MetadataPosition)?;
		match s.block {
			SegmentBlock::Metadata(d) => Ok(d),
			_ => Err(Error::from("Unexpected block when looking for metadata"))
		}
	}
	fn get_directory(&self,file:&Arc<dyn FileExt>) -> Result<Directory>{
		let s:Segment = Segment::new(file, self.DirectoryPosition)?;
		match s.block {
			SegmentBlock::Directory(d) => Ok(d),
			_ => Err(Error::from("Unexpected block when looking for directory"))
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
