mod error;

use std::borrow::Borrow;
use chrono::{DateTime, Local, TimeZone};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::path::{Path,PathBuf};
use std::sync::Arc;
use rusqlite::Connection;
use rusqlite::types::FromSql;
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
		timestamp integer,
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
	fn has_file(&self, filename:&Path) -> rusqlite::Result<bool>{
		self.conn.prepare("SELECT filename FROM files WHERE filename=?")?
			.exists([filename.to_str().unwrap()])
	}
	fn register_image(&self, hd:&zisraw::structs::FileHeader, file:&Arc<dyn FileExt>) -> Result<RegisterSuccess>{
		if !self.has_image(&hd.FileGuid)?
		{ // image is not yet known, register it
			let mut metadata = hd.get_metadata(file)?;
			let metadata_tree = metadata.as_tree()?;

			let org_filename = metadata_tree
				.drill_down(["Experiment", "ImageName"].borrow())?
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
				INSERT INTO images (guid, parent_guid, file_part, timestamp, original_path, meta_data, thumbnail_type, thumbnail) \
				values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)\
			",
				(
					hd.FileGuid.to_string(),
					primary_file_guid,
					hd.FilePart,
					hd.get_timestamp(file)?.timestamp(), // decode with SELECT datetime(acquisition_timestamp, 'unixepoch')
					org_filename,
					metadata.cache.source,
					thumbnail_type, thumbnail_data
				)
			)?;
			Ok(RegisterSuccess::Inserted)
		} else {//image is already registered but filename is new
			let existing = self.lookup_filenames(&hd.FileGuid)?;
			Ok(RegisterSuccess::ImageExists(existing))
		}
	}
	pub fn query_from_images<T:FromSql,O:Into<Option<String>>>(&self, column:&str, where_clause:O) -> Result<Vec<T>>{
		let where_clause:Option<String> = where_clause.into();
		let query = match where_clause{
			None => format!("SELECT {column} FROM images"),
			Some(c) => format!("SELECT {column} FROM images WHERE {}",c)
		};
		let mut stmt= self.conn.prepare(query.as_str())?;
		let rows = stmt.query_map([],|r| {r.get::<usize,T>(0)})?;
		Ok(rows.filter_map(|r|r.ok()).collect())
	}
	pub fn query_images<O:Into<Option<String>>>(&self,where_clause:O) -> Result<Vec<ImageInfo>>{
		let where_clause:Option<String> = where_clause.into();// e.g. "acquisition_timestamp < strftime('%s','now')"
		let query = match where_clause{
			None => "SELECT guid, parent_guid, file_part, timestamp, original_path FROM images".to_string(),
			Some(c) => format!("SELECT guid, parent_guid, file_part, timestamp, original_path FROM images WHERE {}",c)
		};
		let mut stmt= self.conn.prepare(query.as_str())?;
		// todo implement proper error handling (right now we simply ignore rows that raised errors
		let rows = stmt.query_map([],|r| {
			let guid = guid_from_string(r.get(0)?).unwrap_or_default();
			Ok(ImageInfo {
				timestamp: Local.timestamp(r.get(3)?,0),
				guid,
				parent_guid: guid_from_maybe_string(r.get(1)?).unwrap_or_default(),
				orig_path: r.get(4).and_then(|v: String| Ok(PathBuf::from(v)))?,
				file_part: r.get(2)?,
				filenames: self.lookup_filenames(&guid)?
			})
		})?;
		Ok(rows.filter_map(|r|r.ok()).collect())
	}
	pub fn get_image(&self,id:Uuid)-> Result<Option<ImageInfo>>{
		let found = self.query_images(format!("guid = \"{id}\""));
		found.map(|mut v|v.pop())
	}
	pub fn get_image_thumbnail(&self,id:Uuid)-> Result<Option<Vec<u8>>>{
		let found = self.query_from_images("thumbnail", format!("guid = \"{id}\""));
		found.map(|mut v|v.pop())
	}
	pub fn get_image_xml(&self,id:Uuid)-> Result<Option<String>>{
		let found = self.query_from_images("meta_data", format!("guid = \"{id}\""));
		found.map(|mut v|v.pop())
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
	pub fn register_file(&self, filename:&Path) -> Result<RegisterSuccess>{
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

