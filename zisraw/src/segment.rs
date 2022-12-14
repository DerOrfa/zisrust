use std::io::{Error, ErrorKind};
use iobase::blockbuf::{BlockBuf, BlockRead};
use std::sync::Arc;
use std::os::unix::fs::FileExt;
use iobase::Endian::Little;
use crate::Result;
use super::structs::*;
use uuid::Uuid;
use xmltree::Element;
use iobase::{basic::Cached,DataFromFile};

pub fn parse_xml(source:&String) ->Result<Element>{
	Element::parse(source.as_bytes()).or_else(|e|Err(e.into()))
}

#[derive(Debug)]
pub struct Segment{
	pub allocated_size:u64,
	pub used_size:u64,
	pub pos:u64,
	pub block:SegmentBlock,
}

impl Segment{
	pub fn new(file:&Arc<dyn FileExt>,pos:u64) -> std::io::Result<Self>{
		//create buffer block beginning with the segment
		let mut buffer=BlockBuf::new(file.clone(),pos,Little)?;// prepare and read 1k for now
		// get header from there
		let id= buffer.get_ascii::<16>()?;
		let allocated_size = buffer.get_scalar()?;
		let used_size = buffer.get_scalar()?;

		// now that we know the segments size we go back an resize the buffer which might read more data
		buffer.resize(32+allocated_size as usize)?;

		let s = Segment{
			pos,
			allocated_size,
			used_size: {if used_size==0 {allocated_size} else {used_size}},
			block: match id.as_str() {
				"ZISRAWFILE" => SegmentBlock::FileHeader(buffer.read()?),
				"ZISRAWATTDIR" => SegmentBlock::AttachmentDirectory(buffer.read()?),
				"ZISRAWMETADATA" => SegmentBlock::Metadata(buffer.read()?),
				"ZISRAWSUBBLOCK" => SegmentBlock::ImageSubBlock(buffer.read()?),
				"ZISRAWDIRECTORY" => SegmentBlock::Directory(buffer.read()?),
				"ZISRAWATTACH" => SegmentBlock::Attachment(buffer.read()?),
				_ => SegmentBlock::DELETED
			}
		};
		Ok(s)
	}
}

#[derive(Debug)]
pub enum SegmentBlock{
	// File Header segment, occurs only once per file. The segment is always located at position 0.
	FileHeader(FileHeader),
	// Directory segment containing a sequence of "DirectoryEntry" items.
	Directory(Directory),
	// Contains an ImageSubBlock containing an XML part, optional pixel data and binary attachments described
	// by the AttachmentSchema within the XML part .
	ImageSubBlock(SubBlock),
	// Contains Metadata consisting of an XML part and binary attachments described by the AttachmentSchema
	// within the XML part.
	Metadata(Metadata),
	// Any kind of named Attachment, some names are reserved for internal use.
	Attachment(Attachment),
	// Attachments directory.
	AttachmentDirectory(AttachmentDirectory),
	// Indicates that the segment has been deleted (dropped) and should be skipped or ignored by readers.
	DELETED
}

impl BlockRead for FileHeader{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self> {
		let version = buffer.get_array()?;
		buffer.skip_to(16).unwrap();
		let PrimaryFileGuid = Uuid::from_slice(buffer.drain(16)?.as_slice()).unwrap();//impossible to fail
		let FileGuid = Uuid::from_slice(buffer.drain(16)?.as_slice()).unwrap();
		Ok(FileHeader{
			version, PrimaryFileGuid, FileGuid,
			FilePart: buffer.get_scalar()?,
			DirectoryPosition: buffer.get_scalar()?,
			MetadataPosition: buffer.get_scalar()?,
			UpdatePending: buffer.get_scalar::<i32>()?!=0,
			AttachmentDirectoryPosition: buffer.get_scalar()?
		})
	}
}

impl BlockRead for Metadata{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self> {
		let xml_size:i32= buffer.get_scalar()?;
		match buffer.skip_to(256)?.get_utf8(xml_size as usize){
			Ok(s) => Ok(Metadata{cache: Cached::new(s, parse_xml)}),
			Err(e) => Err(Error::new(ErrorKind::InvalidData,"Failed to read xml string"))
		}
	}
}

impl BlockRead for SubBlock{
	fn read(buffer:&mut BlockBuf) -> std::io::Result<Self> {
		let metadata_size:u32 = buffer.get_scalar()?;
		let attachment_size:u32= buffer.get_scalar()?;
		let data_size:u64 = buffer.get_scalar()?;
		let Entry = buffer.read()?;

		buffer.skip_to(256).ok();
		let Metadata = match buffer.get_utf8(metadata_size as usize){
			Ok(s) => Cached::new(s, parse_xml),
			Err(e) => return Err(Error::new(ErrorKind::InvalidData,"Failed to read xml string"))
		};

		let Data = buffer.get_cached_data(data_size as usize);

		let Attachment:Option<DataFromFile> =
			if attachment_size>0 {
				Some(buffer.get_cached_data(attachment_size as usize))
			} else {
				None
			};
		Ok(SubBlock{Entry,Metadata, Data, Attachment})
	}
}

impl BlockRead for DimensionEntryDV1{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self> {
		Ok(DimensionEntryDV1{
			Dimension: buffer.get_ascii::<4>()?,
			Start: buffer.get_scalar()?,
			Size: buffer.get_scalar()?,
			StartCoordinate: buffer.get_scalar()?,
			StoredSize: buffer.get_scalar()?
		})
	}
}

impl BlockRead for DirectoryEntryDV{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self>{
		let SchemaType= buffer.get_ascii::<2>()?;
		let PixelType = buffer.get_scalar()?;
		let FilePosition = buffer.get_scalar()?;
		let FilePart = buffer.get_scalar()?;
		let Compression = buffer.get_scalar()?;
		let PyramidType = buffer.get_scalar()?;//PyramidType, and 5 reserved bytes
		let dimension_count:u32 = buffer.skip_to(28)?.get_scalar()?;
		let dimension_map = buffer
			.read_vec(dimension_count as usize)?
			.into_iter()
			.map(|de:DimensionEntryDV1|(de.Dimension.clone(),de))
			.collect();
		Ok(DirectoryEntryDV{SchemaType, PixelType, FilePosition, FilePart, Compression, PyramidType, dimension_map})
	}
}

impl BlockRead for Directory{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self> where Self: Sized {
		let EntryCount:i32 = buffer.get_scalar()?;
		Ok(Directory{
			Entries:buffer.skip_to(128)?.read_vec(EntryCount as usize)?
		})
	}
}

impl BlockRead for Attachment{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self> {
		let data_size:u32 = buffer.get_scalar()?;
		Ok(Attachment{
			Entry:buffer.skip_to(16)?.read()?,
			Data:buffer.skip_to(256)?.get_cached_data(data_size as usize)
		})
	}
}

impl BlockRead for AttachmentEntryA1{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self> {
		let SchemaType = buffer.get_ascii::<2>()?;
		buffer.skip_to(12).expect("Unexpected backward skip");
		let FilePosition=buffer.get_scalar()?;
		let FilePart=buffer.get_scalar()?;
		let ContentGuid=Uuid::from_slice(buffer.drain(16)?.as_slice()).unwrap(); // impossible to fail
		Ok(AttachmentEntryA1{
			SchemaType, // todo implement support for attachments other than thumbnails
			FilePosition, FilePart,	ContentGuid,
			ContentFileType: buffer.get_ascii::<8>()?,
			Name: buffer.get_ascii::<80>()?
		})
	}
}

impl BlockRead for AttachmentDirectory{
	fn read(buffer: &mut BlockBuf) -> std::io::Result<Self> where Self: Sized {
		let count:u32 = buffer.get_scalar()?;
		buffer.skip_to(256).expect("Unexpected backward skip");
		Ok(AttachmentDirectory {Entries:buffer.read_vec(count as usize)?})
	}
}
