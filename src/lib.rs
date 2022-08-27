pub mod io;
pub mod utils;
pub mod pyramid;
pub mod db;

use std::path::PathBuf;
use chrono::{DateTime, Local};
use uuid::Uuid;

use serde::{Deserialize, Serialize};

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct ImageInfo{
	pub timestamp:DateTime<Local>,
	pub guid:Uuid,
	pub parent_guid:Option<Uuid>,
	pub orig_path:PathBuf,
	pub file_part:i32,
	pub filenames:Vec<PathBuf>
}
