use std::error::Error;
use std::path::{Path, PathBuf};
use rusqlite::{params, Connection, Result};
use uuid::Uuid;

#[derive(Debug)]
struct CziFile {
	FileGuid:Uuid, //Unique Per file
	PrimaryFileGuid:Option<Uuid>,//Unique Guid of Master file (FilePart 0)
	FilePart:i32, // Part number in multi-file scenarios
	original_path: PathBuf,
	meta_data:String
}

fn main() -> Result<(), Box<dyn Error>> {
	let conn = Connection::open("czi_registry.db")?;

	conn.execute(
		"create table if not exists files (
             FileGuid text primary key, PrimaryFileGuid text, FilePart integer, original_path string, meta_data string
         )",
		[],
	)?;
	Ok(())
}
