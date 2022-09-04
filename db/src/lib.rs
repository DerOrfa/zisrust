mod error;

use std::borrow::Borrow;
use chrono::{DateTime, Local};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::sync::Arc;
use rusqlite::Connection;
use uuid::Uuid;
use uuid;
use zisraw::utils::XmlUtil;
use zisraw::ZisrawInterface;
use serde::{Deserialize, Serialize};
pub use error::Error;
pub use iobase::Result;

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
	ImageExists(Vec<PathBuf>),
	FileExists
}

pub struct DB {
	conn:Connection
}

#[derive(Serialize, Deserialize)]
pub struct ImageInfo{
	pub timestamp:DateTime<Local>,
	pub guid:Uuid,
	pub parent_guid:Option<Uuid>,
	pub orig_path:PathBuf,
	pub file_part:i32,
	pub filenames:Vec<PathBuf>
}

fn guid_from_string(s:String)->Result<Uuid>{
	Uuid::parse_str(s.as_str())
		.or_else(|e|Err(Error::Because((e.into(),format!("Failed to parse {s} as uuid)"))).into()))
}

fn guid_from_maybe_string(s:Option<String>)->Result<Option<Uuid>>{
	s.clone()
		.map(|x:String|Uuid::parse_str(x.as_str()))
		.transpose()
		.or_else(
			|e|Err(Error::Because((
				e.into(),
				format!("Failed to parse {} as uuid)",s.unwrap())
			)).into())
		)
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
	fn register_image(&self, hd:&zisraw::structs::FileHeader, file:&Arc<dyn FileExt>) -> Result<RegisterSuccess>{
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

			let mut thumbnail = hd.get_thumbnail(file)?;
			let thumbnail_type = thumbnail.as_ref().map(|t|t.Entry.ContentFileType.clone());
			let thumbnail_data= match thumbnail.as_mut() {
				Some(a) => Some(a.Data.get()?),
				_ => None
			};

			self.conn.execute(
				"\
				INSERT INTO images (guid, parent_guid, file_part, acquisition_timestamp, original_path, meta_data, thumbnail_type, thumbnail) \
				values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)\
			",
				(
					hd.FileGuid.to_string(),
					primary_file_guid,
					hd.FilePart,
					acquisition_timestamp,
					org_filename,
					metadata.cache.source,
					thumbnail_type,thumbnail_data
				)
			)?;
			Ok(RegisterSuccess::Inserted)
		} else {//image is already registered but filename is new
			let existing = self.lookup_filenames(&hd.FileGuid)?;
			Ok(RegisterSuccess::ImageExists(existing))
		}
	}
	pub fn query_images(&self,where_clause:Option<String>) -> Result<Vec<ImageInfo>>
	{
		let query = match where_clause{
			None => "SELECT guid, parent_guid, file_part, acquisition_timestamp, original_path FROM images".to_string(),
			Some(c) => format!("SELECT guid, parent_guid, file_part, acquisition_timestamp, original_path FROM images WHERE {}",c)
		};
		let mut stmt= self.conn.prepare(query.as_str())?;
		// todo implement proper error handling (right now we simply ignore rows that raised errors
		let rows = stmt.query_map([],|r| {
			let guid = guid_from_string(r.get(0)?).unwrap_or_default();
			Ok(ImageInfo {
				timestamp: r.get(3)?,
				guid,
				parent_guid: guid_from_maybe_string(r.get(1)?).unwrap_or_default(),
				orig_path: r.get(4).and_then(|v: String| Ok(PathBuf::from(v)))?,
				file_part: r.get(2)?,
				filenames: self.lookup_filenames(&guid)?
			})
		})?;
		Ok(rows.filter_map(|r|r.ok()).collect())
	}
	pub fn new(filename:&PathBuf) -> rusqlite::Result<Self> {
		let slf=Self{
			conn: Connection::open(filename)?
		};
		slf.conn.execute(IMAGE_TABLE_CREATE, [])?;
		slf.conn.execute(FILE_TABLE_CREATE, [])?;
		Ok(slf)
	}
	pub fn lookup_filenames(&self,guid:&Uuid) -> rusqlite::Result<Vec<PathBuf>>{
		self.conn.prepare("SELECT filename FROM files WHERE image_id = ?")?
			.query_map([guid.to_string()],|row|
				row.get(0).and_then(|v: String| Ok(PathBuf::from(v)))
			)?.collect()
	}
	pub fn register_file(&self, filename:&PathBuf) -> Result<RegisterSuccess>{
		if self.has_file(&filename)?{
			return Ok(RegisterSuccess::FileExists);//file is already registered
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

