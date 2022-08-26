#![allow(non_snake_case)]
use std::fs::File;
use crate::io::{FileRead, FileGet, Endian, Data, basic::Cached};
use crate::io::zisraw::zisraw_structs::*;
use std::io::{Read, Seek, SeekFrom, ErrorKind::InvalidData, Error, Result, BufReader};
use euclid::Rect;
use xmltree::Element;
use crate::pyramid;
use super::ZisrawInterface;

fn skip<T:Read+Seek>(file:&mut T, bytes:u64)-> std::io::Result<u64>{
	if bytes > 3 * 1024 {
		file.seek(SeekFrom::Current(bytes as i64))
	} else {
		std::io::copy(&mut file.by_ref().take(bytes), &mut std::io::sink())
	}
}

pub fn parse_xml(source:&String) ->Element{
	Element::parse(source.as_bytes()).unwrap()
}

impl Segment{
	fn skip_to_start<T:Read+Seek>(&self, file: &mut T,offset:u64)->Result<u64>{
		let to = self.pos+32+offset;//allocated_size starts after the segment header which is 32bytes
		let current= file.stream_position()?;
		if to < current {
			file.seek(SeekFrom::Start(to))
		} else {
			skip(file, to - current)
		}
	}
	fn skip_to_end<T:Read+Seek>(&self, file: &mut T) -> Result<u64>{
		self.skip_to_start(file,self.allocated_size)
	}
}

impl FileRead<BufReader<File>> for Segment{
	fn read(file: &mut BufReader<File>, endianess: &Endian) -> Result<Self> {
		let pos = file.stream_position()?;
		let id= file.get_ascii::<16>()?;
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
		let count:u32 = file.get(endianess)?;
		skip(file, 252)?;
		Ok(AttachmentDirectory{
			Entries: file.get_vec(count as usize,endianess)?
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
		let xml_size:i32= file.get(endianess)?;
		skip(file,256-4)?;//actually there is also 4 bytes reserved for AttachmentSize, but that's "NOT USED CURRENTLY"
		let xml_string=file.get_utf8(xml_size as u64)?;
		Ok(Metadata{
			cache: Cached::new(xml_string, parse_xml)
		})
	}
}

impl FileRead<BufReader<File>> for SubBlock{
	fn read(file: &mut BufReader<File>, endianess: &Endian) -> Result<Self> {
		let skip_to=file.stream_position()?+256;
		let metadata_size:u32 = file.get(endianess)?;
		let attachment_size:u32= file.get(endianess)?;
		let data_size:u64 = file.get(endianess)?;
		let Entry = file.get(endianess)?;
		let current_pos= file.stream_position()?;
		if skip_to>current_pos{skip(file,skip_to-current_pos)?;}

		let metadata_xml = file.get_utf8(metadata_size as u64)?;
		let Metadata = Cached::new(metadata_xml, parse_xml);
		let Data = Data::new(file,data_size as usize)?;
		let Attachment:Option<Data> = if attachment_size>0 {Some(Data::new(file,attachment_size as usize)?)} else {None};
		Ok(SubBlock{Entry,Metadata,	Data, Attachment,})
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
		let dimension_count:u32 = file.get(endianess)?;
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
		let EntryCount:i32 = file.get(endianess)?;
		skip(file,124)?;
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

impl FileRead<BufReader<File>> for Attachment{
	fn read(file: &mut BufReader<File>, endianess: &Endian) -> Result<Self> {
		let size:u32 = file.get(endianess)?;
		skip(file,12)?;
		let Entry = file.get(endianess)?;
		skip(file,112)?;
		let Data = Data::new(file,size as usize)?;

		Ok(Attachment{Entry,Data})
	}
}

impl ZisrawInterface for FileHeader{
	fn get_metadata(&self,file:&mut BufReader<File>) -> Result<Metadata>{
		file.seek(SeekFrom::Start(self.MetadataPosition))?;
		let s:Segment = file.get(&Endian::Little)?;
		match s.block {
			SegmentBlock::Metadata(d) => Ok(d),
			_ => Err(Error::new(InvalidData,"Unexpected block when looking for metadata"))
		}
	}
	fn get_directory(&self,file:&mut BufReader<File>) -> Result<Directory>{
		file.seek(SeekFrom::Start(self.DirectoryPosition))?;
		let s:Segment = file.get(&Endian::Little)?;
		match s.block {
			SegmentBlock::Directory(d) => Ok(d),
			_ => Err(Error::new(InvalidData,"Unexpected block when looking for directory"))
		}
	}
	fn get_attachments(&self,file:&mut BufReader<File>)-> Result<Vec<AttachmentEntryA1>>{
		file.seek(SeekFrom::Start(self.AttachmentDirectoryPosition))?;
		let s:Segment = file.get(&Endian::Little)?;
		let attachments= match s.block {
			SegmentBlock::AttachmentDirectory(d) => Ok(d),
			_ => Err(Error::new(InvalidData,"Unexpected block when looking for directory"))
		}?;
		Ok(attachments.Entries)
	}
}
