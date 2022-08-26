use std::borrow::Borrow;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use rusqlite::Connection;
use uom::si::volume::Units::register_ton;
use uuid::Uuid;
use xmltree::Error::Io;
use crate::db::RegisterResult::{ImageExists,FileExists,SqlErr,IoErr};
use crate::io::{FileGet, zisraw};
use crate::io::zisraw::zisraw_structs::{Segment, SegmentBlock};
use crate::io::zisraw::ZisrawInterface;
use crate::utils::XmlUtil;

const IMAGE_TABLE_CREATE: &'static str =
	r#"create table if not exists images (
		guid CHAR(36) primary key,
		parent_guid CHAR(36),
		file_part integer,
		acquisition_timestamp integer,
		original_path string,
		meta_data string
	)"#;

const FILE_TABLE_CREATE: &'static str =
	r#"create table if not exists files (
		filename TEXT NOT NULL PRIMARY KEY,
		image_id CHAR(36) NOT NULL,
		md5sum CHAR(32),
		FOREIGN KEY (image_id) REFERENCES images (guid)
	)"#;

pub enum RegisterResult{
	Ok,
	ImageExists(Vec<String>),
	FileExists,
	SqlErr(rusqlite::Error),
	IoErr(std::io::Error)
}

pub fn init_db(filename:&PathBuf) -> rusqlite::Result<Connection>{
	let conn = Connection::open(filename)?;
	conn.execute(IMAGE_TABLE_CREATE, [])?;
	conn.execute(FILE_TABLE_CREATE, [])?;
	Ok(conn)
}

pub fn lookup_filenames(conn:&Connection, guid:&Uuid) -> rusqlite::Result<Vec<String>>{
	conn.prepare("SELECT filename FROM files WHERE image_id = ?")?
		.query_map([guid.to_string()],|row|row.get(0))?
		.collect()
}

pub fn has_image(conn:&Connection, guid:&Uuid) -> rusqlite::Result<bool>{
	conn.prepare("SELECT guid FROM images WHERE guid=?")?
		.exists([guid.to_string()])
}

pub fn has_file(conn:&Connection, filename:&PathBuf) -> rusqlite::Result<bool>{
	conn.prepare("SELECT filename FROM files WHERE filename=?")?
		.exists([filename.to_str().unwrap()])
}

pub fn register_file(conn:&Connection, filename:&PathBuf) -> RegisterResult{
	if match has_file(conn,&filename){Ok(b) => b, Err(e) => return SqlErr(e)}{
		return FileExists;//file is already registered
	}
	let mut file = match File::open(filename.clone()){
		Ok(f) => BufReader::new(f),
		Err(e) => return IoErr(e)
	};
	let hd = match zisraw::get_file_header(&mut file){
		Ok(hd) => hd,
		Err(e) => return IoErr(e)
	};

	let result = if !match has_image(conn,&hd.FileGuid){Ok(b) => b,Err(e) => return SqlErr(e)}
	{ // image is not yet known, register it
		let mut metadata = match hd.get_metadata(&mut file){
			Ok(m) => m,
			Err(e) => return IoErr(e)
		};
		let mut metadata_tree = match metadata.as_tree(){
			Ok(m) => m,
			Err(e) => return IoErr(e)
		};

		let image_branch = metadata_tree
			.take_child("Information").unwrap()
			.take_child("Image").unwrap();
		let acquisition_timestamp: chrono::DateTime<chrono::Local> =
			match image_branch.child_into("AcquisitionDateAndTime"){
				Ok(v) => v,
				Err(e) => return IoErr(e)
			};

		let org_filename =
			match metadata_tree.drill_down(["Experiment", "ImageName"].borrow()){
				Ok(e) => e,
				Err(e) => return IoErr(e)
			}.get_text().unwrap();
		let primary_file_guid = if hd.PrimaryFileGuid == hd.FileGuid { None } else { Some(hd.PrimaryFileGuid.to_string()) };

		let thumbnail = match hd.get_attachments(&mut file){
			Ok(a) => a,
			Err(e) => return IoErr(e)
		}.into_iter().filter(|a|a.Name=="Thumbnail").next();

		if thumbnail.is_some(){
			match file.seek(SeekFrom::Start(thumbnail.unwrap().FilePosition)){Err(e) => return IoErr(e),_ => {}};
			let att:Segment = match file.get(&crate::io::Endian::Little){
				Ok(v) => v,
				Err(e) => return IoErr(e)
			};
			let att= match att.block{
				SegmentBlock::Attachment(a) => a,
				_ => return IoErr(std::io::Error::new(std::io::ErrorKind::InvalidInput,"Unexpected block when looking for attachment"))
			};
			println!("{att:?}");
		}

		match conn.execute(
			"\
				INSERT INTO images (guid, parent_guid, file_part, acquisition_timestamp, original_path, meta_data) \
				values (?1, ?2, ?3, ?4, ?5, ?6)\
			",
			(hd.FileGuid.to_string(), primary_file_guid, hd.FilePart, acquisition_timestamp.timestamp(), org_filename, metadata.cache.source)
		){
			Ok(_) => RegisterResult::Ok,//image was registered
			Err(e) => return SqlErr(e)
		}
	} else {//image is already registered but filename is new
		let existing = lookup_filenames(conn,&hd.FileGuid);
		match existing{
			Ok(v) => ImageExists(v),
			Err(e) => SqlErr(e)
		}
	};

	// register filename regardless if image was new and return either result of registration or error
	match conn.execute(
		"\
				INSERT INTO files (filename, image_id) \
				values (?1, ?2)\
			",
		(filename.to_str().unwrap(), hd.FileGuid.to_string())
	){
		Ok(_) => result,
		Err(e) => SqlErr(e)
	}



}

