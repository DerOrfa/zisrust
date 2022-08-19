#![allow(non_snake_case)]

use std::fs::File;
use std::io::{BufReader, Seek};
use uuid::Uuid;
use memmap::Mmap;
use xmltree::Element;
use std::io::{SeekFrom,Result,Error,ErrorKind};
use crate::io::{FileGet,Endian};
use crate::io::basic::Cached;

pub fn get_file_header(file:&mut BufReader<File>) -> Result<FileHeader>{
	file.seek(SeekFrom::Start(0))?;
	let s:Segment = file.get(&Endian::Little)?;
	match s.block {
		SegmentBlock::FileHeader(hd) => Ok(hd),
		_ => Err(Error::new(ErrorKind::InvalidInput,"Unexpected block when looking for header"))
	}
}

#[derive(Debug)]
pub struct Segment{
	pub allocated_size:u64,
	pub used_size:u64,
	pub pos:u64,
	pub block:SegmentBlock
}

#[derive(Debug)]
pub struct Data{
	pub cache: Cached<Mmap,Vec<u8>>
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
	pub AttachmentDirectoryPosition:i64
}

#[derive(Debug)]
pub struct Directory{
	pub EntryCount:i32,
	pub Reserved:[u8;124],
	pub Entries:Vec<DirectoryEntryDV>
}

#[derive(Debug)]
pub struct Metadata{
	//pub AttachmentSize:i32, //NOT USED CURRENTLY
	pub cache:Cached<String,Element>
}

#[derive(Debug)]
pub struct AttachmentDirectory{
	pub Entries:Vec<AttachmentEntryA1>
}

#[derive(Debug)]
pub struct Attachment{
	pub Entry:AttachmentEntryA1,
	pub Data:Data
}

#[derive(Debug)]
pub struct AttachmentEntryA1{
	pub SchemaType:String, //rad as [char;2]
	pub Reserved:[char;10],
	pub FilePosition:i64,
	pub FilePart:i32,
	pub ContentGuid:Uuid,
	pub ContentFileType:String, //read as [char;8]
	pub Name:String //read as [char;80]
}

#[derive(Debug)]
pub struct SubBlock {
	pub Entry:DirectoryEntryDV,
	pub Metadata: Cached<String,Element>,
	pub Data:Data,
	pub Attachment:Option<Data>
}

#[derive(Debug)]
pub struct DirectoryEntryDV{
	pub SchemaType:String,//read as [char;4]
	pub PixelType:i32,
	pub FilePosition:i64,
	pub FilePart:i32,
	pub Compression:i32,
	pub PyramidType:u8,
	pub DimensionEntries:Vec<DimensionEntryDV1>,
}

#[derive(Debug)]
pub struct DimensionEntryDV1{
	pub Dimension:String,//read as [char;4]
	pub Start:i32,
	pub Size:u32,
	pub StartCoordinate:f32,
	pub StoredSize:i32
}
