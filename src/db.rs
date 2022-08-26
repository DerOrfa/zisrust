use std::borrow::Borrow;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::sync::Arc;
use rusqlite::Connection;
use uuid::Uuid;
use crate::db::RegisterSuccess::{FileExists, ImageExists, Inserted};
use crate::io::zisraw;
use crate::io::zisraw::{zisraw_structs,ZisrawInterface};
use crate::utils::XmlUtil;

const IMAGE_TABLE_CREATE: &'static str =
	r#"create table if not exists images (
		guid CHAR(36) primary key,
		parent_guid CHAR(36),
		file_part integer,
		acquisition_timestamp integer,
		original_path string,
		meta_data string,
		thumbnail_type string,
		thumbnail blob
	)"#;

const FILE_TABLE_CREATE: &'static str =
	r#"create table if not exists files (
		filename TEXT NOT NULL PRIMARY KEY,
		image_id CHAR(36) NOT NULL,
		md5sum CHAR(32),
		FOREIGN KEY (image_id) REFERENCES images (guid)
	)"#;

pub enum RegisterSuccess{
	Inserted,
	ImageExists(Vec<String>),
	FileExists
}
type RegisterError = Box<dyn std::error::Error>;
pub type RegisterResult = Result<RegisterSuccess,RegisterError>;

pub struct DB {
	conn:Connection
}

impl DB {
	fn has_image(&self, guid:&Uuid) -> rusqlite::Result<bool>{
		self.conn.prepare("SELECT guid FROM images WHERE guid=?")?
			.exists([guid.to_string()])
	}
	fn has_file(&self, filename:&PathBuf) -> rusqlite::Result<bool>{
		self.conn.prepare("SELECT filename FROM files WHERE filename=?")?
			.exists([filename.to_str().unwrap()])
	}
	fn register_image(&self,hd:&zisraw_structs::FileHeader, file:&Arc<dyn FileExt>) -> RegisterResult{
		if !self.has_image(&hd.FileGuid)?
		{ // image is not yet known, register it
			let mut metadata = hd.get_metadata(file)?;
			let mut metadata_tree = metadata.as_tree()?;

			let image_branch = metadata_tree
				.take_child("Information").unwrap()
				.take_child("Image").unwrap();
			let acquisition_timestamp: chrono::DateTime<chrono::Local> =
				image_branch.child_into("AcquisitionDateAndTime")?;

			let org_filename =
				metadata_tree.drill_down(["Experiment", "ImageName"].borrow())?
					.get_text().unwrap();
			let primary_file_guid = if hd.PrimaryFileGuid == hd.FileGuid { None } else { Some(hd.PrimaryFileGuid.to_string()) };

			let thumbnail = hd.get_thumbnail(file)?;
			let thumbnail_type = thumbnail.as_ref().map(|t|&t.Entry.ContentFileType);

			self.conn.execute(
				"\
				INSERT INTO images (guid, parent_guid, file_part, acquisition_timestamp, original_path, meta_data, thumbnail_type) \
				values (?1, ?2, ?3, ?4, ?5, ?6, ?7)\
			",
				(
					hd.FileGuid.to_string(),
					primary_file_guid,
					hd.FilePart,
					acquisition_timestamp.timestamp(),
					org_filename,
					metadata.cache.source,
					thumbnail_type
				)
			)?;
			Ok(Inserted)
		} else {//image is already registered but filename is new
			let existing = self.lookup_filenames(&hd.FileGuid)?;
			Ok(ImageExists(existing))
		}
	}
	pub fn new(filename:&PathBuf) -> rusqlite::Result<Self> {
		let slf=Self{
			conn: Connection::open(filename)?
		};
		slf.conn.execute(IMAGE_TABLE_CREATE, [])?;
		slf.conn.execute(FILE_TABLE_CREATE, [])?;
		Ok(slf)
	}
	pub fn lookup_filenames(&self,guid:&Uuid) -> rusqlite::Result<Vec<String>>{
		self.conn.prepare("SELECT filename FROM files WHERE image_id = ?")?
			.query_map([guid.to_string()],|row|row.get(0))?
			.collect()
	}
	pub fn register_file(&self, filename:&PathBuf) -> RegisterResult{
		if self.has_file(&filename)?{
			return Ok(FileExists);//file is already registered
		}
		let file:Arc<dyn FileExt> = Arc::new(File::open(filename.clone())?);
		let hd = zisraw::get_file_header(&file)?;

		let result = self.register_image(&hd,&file)?;

		// register filename regardless if image was new and return either result of registration or error
		self.conn.execute(
			"INSERT INTO files (filename, image_id) values (?1, ?2)",
			(filename.to_str().unwrap(), hd.FileGuid.to_string())
		)?;
		Ok(result)
	}
}

