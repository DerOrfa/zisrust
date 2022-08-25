use std::error::Error;
use std::path::PathBuf;
use rusqlite::{params, Connection, Result};
use uuid::Uuid;
use clap::{Args,Command,arg,crate_version,value_parser};

#[derive(Debug)]
struct CziFile {
	file_guid:Uuid, //Unique Per file
	primary_file_guid:Option<Uuid>,//Unique Guid of Master file (file_part 0)
	file_part:i32, // Part number in multi-file scenarios
	original_path: PathBuf,
	meta_data:String
}

fn main() -> Result<(), Box<dyn Error>> {
	let m = Command::new("czi file registry")
		.author("Enrico Reimer, reimer@cbs.mpg.de")
		.version(crate_version!())
		.about("sqlite backed registry for czi files")
		.arg(arg!(<FILE>).value_parser(value_parser!(PathBuf)))
		.arg(
			arg!(-d --database [DB] "Optionally sets name for the database file")
				.default_value("czi_registry.db")
		)
		.get_matches();

	let db_file = m.get_one::<PathBuf>("DB").unwrap();
	let conn = Connection::open(db_file)?;

	conn.execute(
		"create table if not exists files (
             file_guid text primary key, primary_file_guid text, file_part integer, original_path string, meta_data string
         )",
		[],
	)?;
	Ok(())
}
