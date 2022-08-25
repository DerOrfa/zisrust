use std::borrow::Borrow;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use rusqlite::Connection;
use argh::FromArgs;
use zisrust::io::zisraw;
use zisrust::io::zisraw::ZisrawInterface;
use zisrust::utils::XmlUtil;
use zisrust::db;

#[derive(FromArgs)]
/// Reach new heights.
struct Cli {
	/// an optional nickname for the pilot
	#[argh(option, default = "PathBuf::from(\"czi_registry.db\")")]
	dbfile: PathBuf,
	#[argh(positional)]
	files:Vec<PathBuf>
}
fn main() -> Result<(), Box<dyn Error>> {

	let cli: Cli = argh::from_env();

	let conn = db::init_db(&cli.dbfile)?;

	for fname in cli.files {
		let file = BufReader::new(File::open(fname)?);
		db::register_file(&conn,file)?;
	}
	Ok(())
}
