#![allow(non_snake_case)]

use std::fs::File;
use super::{FileRead, FileGet, Endian};
use super::zisraw::*;
use std::io::{Read, Seek, SeekFrom, ErrorKind, Error, Result, BufReader};
use crate::io::basic::Cached;
use memmap::{Mmap, MmapOptions};
use xmltree::Element;

fn skip<T:Read+Seek>(file:&mut T, bytes:u64)-> Result<u64>{
	if bytes > 3 * 1024 {
		file.seek(SeekFrom::Current(bytes as i64))
	} else {
		std::io::copy(&mut file.by_ref().take(bytes as u64), &mut std::io::sink())
	}
}

impl Data {
	pub fn new(file:&mut BufReader<File>,size:usize)->Data{
		let pos= file.stream_position().unwrap();
		let mmap = unsafe{
			MmapOptions::new()
				.offset(pos)
				.len(size)
				.map(file.get_ref())
		};
		Data{
			size,pos,
			cache: Cached::new(mmap.unwrap(),Self::produce)
		}
	}
	fn produce(source:&Mmap)->Vec<u8>{
		source.to_vec()
	}
}
pub fn parse(source:&String)->Element{
	Element::parse(source.as_bytes()).unwrap()
}

impl Segment{
	fn skip_to_start<T:Read+Seek>(&self, file: &mut T,offset:u64)->std::io::Result<u64>{
		let to = self.pos+32+offset;//allocated_size starts after the segment header which is 32bytes
		let current= file.stream_position()?;
		if to < current {
			file.seek(SeekFrom::Start(to))
		} else {
			skip(file, to - current)
		}
	}
	fn skip_to_end<T:Read+Seek>(&self, file: &mut T) -> std::io::Result<u64>{
		self.skip_to_start(file,self.allocated_size)
	}
}

impl FileRead<BufReader<File>> for Segment{
	fn read(file: &mut BufReader<File>, endianess: &Endian) -> Result<Self> {
		let pos = file.stream_position()?;
		let id=file.get_ascii::<16>()?;
		let allocated_size = file.get(endianess)?;
		let used_size = file.get(endianess)?;

		let s = Segment{
			pos, allocated_size,
			used_size: {if used_size==0 {allocated_size} else {used_size}},
			block: match id.as_str() {
				"ZISRAWFILE" => SegmentBlock::FileHeader(file.get(endianess)?),
				"ZISRAWATTDIR" => SegmentBlock::AttachmentDirectory(file.get(endianess)?),
				"ZISRAWMETADATA" => SegmentBlock::Metadata(file.get(endianess)?),
				"ZISRAWSUBBLOCK" => SegmentBlock::ImageSubBlock(file.get(endianess)?),
				"ZISRAWDIRECTORY" => SegmentBlock::Directory(file.get(endianess)?),
				"ZISRAWATTACH" => SegmentBlock::Attachment(file.get(endianess)?),
				_ => SegmentBlock::DELETED
			}
		};
		s.skip_to_end(file)?;
		if s.used_size==0 {Ok(Segment{used_size:s.allocated_size,..s})}//is used_size is use allocated_size
		else {Ok(s)}
	}
}

impl<T: Read+Seek> FileRead<T> for FileHeader{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(FileHeader{
			version: [file.get(endianess)?,file.get(endianess)?],
			Reserved1: file.get(endianess)?,
			Reserved2: file.get(endianess)?,
			PrimaryFileGuid: file.get(endianess)?,
			FileGuid: file.get(endianess)?,
			FilePart: file.get(endianess)?,
			DirectoryPosition: file.get(endianess)?,
			MetadataPosition: file.get(endianess)?,
			UpdatePending: i32::read(file,endianess)?!=0,
			AttachmentDirectoryPosition: file.get(endianess)?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for AttachmentDirectory{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let EntryCount:i32 = file.get(endianess)?;
		skip(file, 252)?;
		Ok(AttachmentDirectory{
			EntryCount,
			Entries: file.get_vec(EntryCount as usize,endianess)?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for AttachmentEntryA1{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(AttachmentEntryA1{
			SchemaType: file.get_ascii::<2>()?,
			Reserved: file.get(endianess)?,
			FilePosition: file.get(endianess)?,
			FilePart: file.get(endianess)?,
			ContentGuid: file.get(endianess)?,
			ContentFileType: file.get_ascii::<8>()?,
			Name: file.get_ascii::<80>()?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for Metadata{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let XmlSize:i32= file.get(endianess)?;
		let AttachmentSize:i32= file.get(endianess)?;
		skip(file,256-8)?;
		let xml_string=file.get_utf8(XmlSize as u64)?;
		Ok(Metadata{
			XmlSize,
			AttachmentSize,
			xml_string:xml_string.clone(),
			cache: Cached::new(xml_string, parse)
		})
	}
}

impl FileRead<BufReader<File>> for SubBlock{
	fn read(file: &mut BufReader<File>, endianess: &Endian) -> Result<Self> {
		let skip_to=file.stream_position()?+256;
		let MetadataSize = file.get(endianess)?;
		let AttachmentSize= file.get(endianess)?;
		let DataSize = file.get(endianess)?;
		let Entry = file.get(endianess)?;
		let current_pos= file.stream_position()?;
		if skip_to>current_pos{skip(file,skip_to-current_pos)?;}

		Ok(SubBlock{
			MetadataSize, AttachmentSize, DataSize,	Entry,
			Metadata:file.get_utf8(MetadataSize as u64)?,
			Data: Data::new(file,DataSize as usize)
		})
	}
}

impl<T: Read+Seek> FileRead<T> for DirectoryEntryDV{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let SchemaType= file.get_ascii::<2>()?;
		let PixelType = file.get(endianess)?;
		let FilePosition = file.get(endianess)?;
		let FilePart = file.get(endianess)?;
		let Compression = file.get(endianess)?;
		let buffer:[u8;6] = file.get(endianess)?;
		let DimensionCount = file.get(endianess)?;
		Ok(DirectoryEntryDV{
			SchemaType,	PixelType, FilePosition, FilePart, Compression,
			PyramidType:buffer[0],
			DimensionCount,
			DimensionEntries: file.get_vec(DimensionCount as usize,endianess)?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for DimensionEntryDV1{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(DimensionEntryDV1{
			Dimension: file.get_ascii::<4>()?,
			Start: file.get(endianess)?,
			Size: file.get(endianess)?,
			StartCoordinate: file.get(endianess)?,
			StoredSize: file.get(endianess)?
		})
	}
}

impl<T: Read+Seek> FileRead<T> for Directory{
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		let EntryCount = file.get(endianess)?;
		Ok(Directory{
			EntryCount,
			Reserved: file.get(endianess)?,
			Entries: file.get_vec(EntryCount as usize,endianess)?
		})
	}
}

impl FileRead<BufReader<File>> for Attachment{
	fn read(file: &mut BufReader<File>, endianess: &Endian) -> Result<Self> {
		let DataSize = file.get(endianess)?;
		skip(file,12)?;
		let Entry = file.get(endianess)?;
		skip(file,112)?;
		let Data = Data::new(file,DataSize as usize);

		Ok(Attachment{DataSize,Entry,Data})
	}
}

impl FileHeader{
	pub fn get_directory(&self,file:&mut BufReader<File>) -> Result<Directory>{
		file.seek(SeekFrom::Start(self.DirectoryPosition))?;
		let s:Segment = file.get(&Endian::Little)?;
		match s.block {
			SegmentBlock::Directory(d) => Ok(d),
			_ => Err(Error::new(ErrorKind::InvalidInput,"Unexpected block when looking for directory"))
		}
	}
	fn get_metadata(&self,file:&mut BufReader<File>) -> Result<Metadata>{
		file.seek(SeekFrom::Start(self.MetadataPosition))?;
		let mut s:Segment = file.get(&Endian::Little)?;
		match s.block {
			SegmentBlock::Metadata(d) => Ok(d),
			_ => Err(Error::new(ErrorKind::InvalidInput,"Unexpected block when looking for metadata"))
		}
	}
	pub fn get_metadata_element(&self,file:&mut BufReader<File>) -> Result<Element>{
		let mut cache = self.get_metadata(file)?.cache;
		Ok(cache.get().clone())
	}
	pub fn get_metadata_xml(&self,file:&mut BufReader<File>) -> Result<String>{
		let e = self.get_metadata(file)?;
		Ok(e.cache.source.clone())
	}
	pub fn get_pyramid(&self,file:&mut BufReader<File>)-> Result<()>{
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
