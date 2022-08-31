use uuid::Uuid;
use crate::io::DataFromFile;
use xmltree;

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
	pub cache:crate::io::basic::Cached<String,xmltree::Element>
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
