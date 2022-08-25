use std::borrow::Borrow;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use rusqlite::{Connection,Result};
use uuid::Uuid;
use crate::io::zisraw;
use crate::io::zisraw::ZisrawInterface;
use crate::utils::XmlUtil;


pub fn init_db(filename:&PathBuf) -> Result<Connection>{
	let conn = Connection::open(filename)?;
	conn.execute(
		"create table if not exists files (
             file_guid text primary key,
             primary_file_guid text,
             file_part integer,
			 acquisition_timestamp integer,
             original_path string,
             meta_data string
         )",
		[],
	)
	.and(Ok(conn))
}

pub fn register_file(conn:&Connection, mut file:BufReader<File>) -> std::result::Result<(), Box<dyn std::error::Error>>{
	let hd = zisraw::get_file_header(&mut file)?;

	let mut metadata = hd.get_metadata(&mut file)?;
	let mut metadata_tree = metadata.as_tree()?;

	let image_branch = metadata_tree
		.take_child("Information").unwrap()
		.take_child("Image").unwrap();
	let acquisition_timestamp:chrono::DateTime<chrono::Local>= image_branch.child_into("AcquisitionDateAndTime")?;

	let org_filename = metadata_tree.drill_down(["Experiment","ImageName"].borrow())?.get_text().unwrap();
	let primary_file_guid = if hd.PrimaryFileGuid == hd.FileGuid { None } else { Some(hd.PrimaryFileGuid.to_string()) };

	match conn.execute(
		"\
				INSERT INTO files (file_guid, primary_file_guid, file_part, acquisition_timestamp, original_path, meta_data) \
				values (?1, ?2, ?3, ?4, ?5, ?6)\
			",
		(hd.FileGuid.to_string(),primary_file_guid,hd.FilePart,acquisition_timestamp.timestamp(),org_filename,metadata.cache.source)
	){
		Ok(_) => Ok(()),
		Err(e) => std::result::Result::Err(Box::new(e))
	}
}
