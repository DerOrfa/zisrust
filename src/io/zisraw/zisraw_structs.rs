use uuid::Uuid;
use crate::io::Data;

#[derive(Debug)]
pub struct Segment{
	pub allocated_size:u64,
	pub used_size:u64,
	pub pos:u64,
	pub block:SegmentBlock
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
	pub Entries:Vec<DirectoryEntryDV>
}

#[derive(Debug)]
pub struct Metadata{
	//pub AttachmentSize:i32, //NOT USED CURRENTLY
	pub cache:crate::io::basic::Cached<String,xmltree::Element>
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
	pub SchemaType:String, //4 bytes
	pub Reserved:[u8;10],
	pub FilePosition:i64,
	pub FilePart:i32,
	pub ContentGuid:Uuid,
	pub ContentFileType:String, //8 bytes
	pub Name:String //80 bytes
}

#[derive(Debug)]
pub struct SubBlock {
	pub Entry:DirectoryEntryDV,
	pub Metadata: crate::io::basic::Cached<String,xmltree::Element>,
	pub Data:Data,
	pub Attachment:Option<Data>
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
