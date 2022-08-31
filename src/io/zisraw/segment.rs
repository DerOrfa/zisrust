use crate::io::blockbuf::BlockBuf;
use std::sync::Arc;
use std::os::unix::fs::FileExt;
use crate::io::Endian::Little;
use crate::{Result,Error::Own};
use super::zisraw_structs::*;
use uuid::Uuid;
use xmltree::Element;
use crate::io::{Cached,DataFromFile};

pub fn parse_xml(source:&String) ->Result<Element>{
	Element::parse(source.as_bytes())
		.or_else(|e|Err(Own(format!("Error when parsing xml ({e})"))))
}

#[derive(Debug)]
pub struct Segment{
	pub allocated_size:u64,
	pub used_size:u64,
	pub pos:u64,
	pub block:SegmentBlock,
}

impl Segment{
	pub fn new(file:&Arc<dyn FileExt>,pos:u64) -> Result<Self>{
		//create buffer block beginning with the segment
		let mut buffer=BlockBuf::new(file.clone(),pos,Little)?;// prepare and read 1k for now
		// get header from there
		let id= buffer.get_ascii::<16>()?;
		let allocated_size = buffer.get_scalar();
		let used_size = buffer.get_scalar();

		// now that we know the segments size we go back an resize the buffer which might read more data
		buffer.resize(32+allocated_size as usize)?;

		let s = Segment{
			pos,
			allocated_size,
			used_size: {if used_size==0 {allocated_size} else {used_size}},
			block: match id.as_str() {
				"ZISRAWFILE" => SegmentBlock::FileHeader(SegmentData::read(&mut buffer)),
				"ZISRAWATTDIR" => SegmentBlock::AttachmentDirectory(SegmentData::read(&mut buffer)),
				"ZISRAWMETADATA" => SegmentBlock::Metadata(SegmentData::read(&mut buffer)),
				"ZISRAWSUBBLOCK" => SegmentBlock::ImageSubBlock(SegmentData::read(&mut buffer)),
				"ZISRAWDIRECTORY" => SegmentBlock::Directory(SegmentData::read(&mut buffer)),
				"ZISRAWATTACH" => SegmentBlock::Attachment(SegmentData::read(&mut buffer)),
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

pub trait SegmentData{
	fn read(buffer:&mut BlockBuf) -> Self;
	fn read_vec(buffer:&mut BlockBuf, len:usize) -> Vec<Self> where Self : Sized{
		std::iter::from_fn(||Some(Self::read(buffer))).take(len).collect()
	}
}

impl SegmentData for Directory{
	fn read(buffer: &mut BlockBuf) -> Self {
		let EntryCount:i32 = buffer.get_scalar();
		buffer.skip_to(32+128).unwrap();
		Directory{
			Entries: std::iter::from_fn(||Some(DirectoryEntryDV::read(buffer))).take(EntryCount as usize).collect()
		}
	}
}

impl SegmentData for FileHeader{
	fn read(buffer: &mut BlockBuf) -> Self {
		let version = buffer.get_array();
		buffer.skip_to(32+16);
		let PrimaryFileGuid = Uuid::from_slice(buffer.drain(16).as_slice()).unwrap();
		let FileGuid = Uuid::from_slice(buffer.drain(16).as_slice()).unwrap();
		FileHeader{
			version, PrimaryFileGuid, FileGuid,
			FilePart: buffer.get_scalar(),
			DirectoryPosition: buffer.get_scalar(),
			MetadataPosition: buffer.get_scalar(),
			UpdatePending: buffer.get_scalar::<i32>()!=0,
			AttachmentDirectoryPosition: buffer.get_scalar()
		}
	}
}

impl SegmentData for AttachmentDirectory{
	fn read(buffer: &mut BlockBuf) -> Self {
		let count:u32 = buffer.get_scalar();
		buffer.skip_to(32+256).unwrap();
		AttachmentDirectory {
			Entries: std::iter::from_fn(||Some(AttachmentEntryA1::read(buffer))).take(count as usize).collect()
		}
	}
}

impl SegmentData for AttachmentEntryA1{
	fn read(buffer: &mut BlockBuf) -> Self {
		let SchemaType = buffer.get_ascii::<2>().unwrap();
		buffer.skip_to(32+12);
		let FilePosition = buffer.get_scalar();
		let FilePart = buffer.get_scalar();
		let ContentGuid = Uuid::from_slice(buffer.drain(16).as_slice()).unwrap();
		AttachmentEntryA1{
			SchemaType, // todo implement support for attachments other than thumbnails
			FilePosition, FilePart, ContentGuid,
			ContentFileType: buffer.get_ascii::<8>().unwrap(),
			Name: buffer.get_ascii::<80>().unwrap()
		}
	}
}

impl SegmentData for Metadata{
	fn read(buffer: &mut BlockBuf) -> Self {
		let xml_size:i32= buffer.get_scalar();
		buffer.skip_to(32+256);//actually there is also 4 bytes reserved for AttachmentSize, but that's "NOT USED CURRENTLY"
		let xml_string=buffer.get_utf8(xml_size as usize).unwrap();
		Metadata{
			cache: Cached::new(xml_string, parse_xml)
		}
	}
}

impl SegmentData for SubBlock{
	fn read(buffer: &mut BlockBuf) -> Self {
		let metadata_size:u32 = buffer.get_scalar();
		let attachment_size:u32= buffer.get_scalar();
		let data_size:u64 = buffer.get_scalar();
		let Entry = DirectoryEntryDV::read(buffer);

		buffer.skip_to(32+256).ok();

		let metadata_xml = buffer.get_utf8(metadata_size as usize).unwrap();
		let Metadata = Cached::new(metadata_xml, parse_xml);

		let Data = buffer.get_cached_data(data_size as usize);

		let Attachment:Option<DataFromFile> =
			if attachment_size>0 {
				Some(buffer.get_cached_data(attachment_size as usize))
			} else {
				None
			};
		SubBlock{Entry,Metadata, Data, Attachment}
	}
}

impl SegmentData for DimensionEntryDV1{
	fn read(buffer: &mut BlockBuf) -> Self {
		DimensionEntryDV1{
			Dimension: buffer.get_ascii::<4>().unwrap(),
			Start: buffer.get_scalar(),
			Size: buffer.get_scalar(),
			StartCoordinate: buffer.get_scalar(),
			StoredSize: buffer.get_scalar()
		}
	}
}

impl SegmentData for DirectoryEntryDV{
	fn read(buffer: &mut BlockBuf) -> Self {
		let SchemaType= buffer.get_ascii::<2>().unwrap();
		let PixelType = buffer.get_scalar();
		let FilePosition = buffer.get_scalar();
		let FilePart = buffer.get_scalar();
		let Compression = buffer.get_scalar();
		let PyramidType = buffer.get_scalar();//PyramidType, and 5 reserved bytes
		buffer.skip_to(32+28);
		let dimension_count:u32 = buffer.get_scalar();
		let dimension_map = std::iter::from_fn(||Some(DimensionEntryDV1::read(buffer)))
			.take(dimension_count as usize)
			.map(|de|(de.Dimension.clone(),de))
			.collect();
		DirectoryEntryDV{SchemaType, PixelType, FilePosition, FilePart, Compression, PyramidType, dimension_map}
	}
}

impl SegmentData for Attachment{
	fn read(buffer: &mut BlockBuf) -> Self {
		let data_size:u32 = buffer.get_scalar();
		buffer.skip_to(32+16).unwrap();
		let Entry = SegmentData::read(buffer);
		buffer.skip_to(32+256).unwrap();
		let Data = buffer.get_cached_data(data_size as usize);
		Attachment{Entry,Data}
	}
}

