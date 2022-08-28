use uuid::Uuid;
use crate::io::{DataFromFile, Endian, FileGet};
use xmltree;
use std::io::{Cursor, Seek, SeekFrom};
use std::os::unix::prelude::FileExt;
use std::sync::Arc;
use crate::{Error, Result};

#[derive(Debug)]
pub struct Segment{
	pub allocated_size:u64,
	pub used_size:u64,
	pub pos:u64,
	pub block:SegmentBlock,
}

impl Segment{
	pub fn new(file:&Arc<dyn FileExt>,pos:u64) -> Result<Self>{
		let endianess = &Endian::Little;

		// prepare and read 1k for now
		let mut buffer=Vec::<u8>::from([0;1024]);
		file.read_exact_at(buffer.as_mut_slice(),pos)?;
		let mut buffer=Cursor::new(buffer);
		// get header from there
		let id= buffer.get_ascii::<16>()?;
		let allocated_size = buffer.get_scalar(endianess)?;
		let used_size = buffer.get_scalar(endianess)?;
		let allocated_size = if allocated_size == 0 {used_size} else {allocated_size};

		// now that we know the segments size we go back an resize the buffer
		let mut buffer= buffer.into_inner();
		buffer.resize(allocated_size as usize,0);
		// and read the remaining data, if there are some
		if allocated_size > 1024 {
			file.read_exact_at(buffer.split_at_mut(1024).1,pos+1024)?;
		}
		// ok now put it back into Cursor
		let mut buffer = Cursor::new(buffer);
		buffer.seek(SeekFrom::Start(32)).unwrap();//and skip the header, we had that already

		let s = Segment{
			pos,
			allocated_size,
			used_size: {if used_size==0 {allocated_size} else {used_size}},
			block: match id.as_str() {
				"ZISRAWFILE" => SegmentBlock::FileHeader(buffer.get_scalar(endianess)?),
				"ZISRAWATTDIR" => SegmentBlock::AttachmentDirectory(buffer.get_scalar(endianess)?),
				"ZISRAWMETADATA" => SegmentBlock::Metadata(buffer.get_scalar(endianess)?),
				"ZISRAWSUBBLOCK" => SegmentBlock::ImageSubBlock(SubBlock::new(&mut buffer,file,pos)?),
				"ZISRAWDIRECTORY" => SegmentBlock::Directory(buffer.get_scalar(endianess)?),
				"ZISRAWATTACH" => SegmentBlock::Attachment(Attachment::new(&mut buffer,file, pos)?),
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

#[derive(Debug)]
pub struct FileHeader{
	pub version:[u32;2],
	pub Reserved1:i32,
	pub Reserved2:i32,
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
	pub cache:crate::io::basic::Cached<String,xmltree::Element>
}

impl Metadata {
	pub fn as_tree(&mut self) -> Result<xmltree::Element> {
		self.cache.get()
			.get_child("Metadata")
			.ok_or(Error::from("\"Metadata\" missing in xml stream"))
			.cloned()
	}
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
	pub Reserved:[u8;10],
	pub FilePosition:u64,
	pub FilePart:i32,
	pub ContentGuid:Uuid,
	pub ContentFileType:String, //8 bytes
	pub Name:String //80 bytes
}

#[derive(Debug)]
pub struct SubBlock{
	pub Entry:DirectoryEntryDV,
	pub Metadata: crate::io::basic::Cached<String,xmltree::Element>,
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
