pub mod io;
pub mod utils;
pub mod pyramid;
pub mod db;

use std::fmt::{Debug, Formatter};
use std::path::PathBuf;
use chrono::{DateTime, Local};
use uuid::Uuid;
use serde::{Deserialize, Serialize, ser::Serializer,ser::SerializeStruct};

#[derive(Serialize, Deserialize)]
pub struct ImageInfo{
	pub timestamp:DateTime<Local>,
	pub guid:Uuid,
	pub parent_guid:Option<Uuid>,
	pub orig_path:PathBuf,
	pub file_part:i32,
	pub filenames:Vec<PathBuf>
}


#[derive(Debug)]
pub enum Error {
	Io(std::io::Error),
	Sqlite(rusqlite::Error),
	Own(String)
}
pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error{
	fn from(e: rusqlite::Error) -> Self {Error::Sqlite(e)}
}
impl From<std::io::Error> for Error{
	fn from(e: std::io::Error) -> Self {Error::Io(e)}
}
impl From<&str> for Error{
	fn from(e: &str) -> Self {Error::Own(e.to_string())}
}

impl std::fmt::Display for Error{
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::Io(e) => std::fmt::Display::fmt(e,f),
			Error::Sqlite(e) => std::fmt::Display::fmt(e,f),
			Error::Own(e) => std::fmt::Display::fmt(e,f)
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::Io(e) => e.source(),
			Error::Sqlite(e) => e.source(),
			Error::Own(_) => None
		}
	}
}

impl Serialize for Error {
	fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer,
	{
		let source = std::error::Error::source(self);
		let mut s = serializer.serialize_struct("zisraw error", match source {None => 2,Some(_) => 3})?;
		match self {
			Error::Io(_) => s.serialize_field("type", "io error")?,
			Error::Sqlite(_) => s.serialize_field("type", "sql error")?,
			Error::Own(_) => s.serialize_field("type", "zisraw error")?
		}
		s.serialize_field("source", self.to_string().as_str())?;
		if source.is_some(){
			s.serialize_field("source", source.unwrap().to_string().as_str())?;
		}
		s.end()
	}
}


