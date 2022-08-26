use std::error::Error;
use std::path::PathBuf;
use argh::FromArgs;
use zisrust::db;
use zisrust::db::RegisterResult;

#[derive(FromArgs)]
#[argh(description = "sqlite backed registry for czi files")]
struct Cli {
	#[argh(option, default = "PathBuf::from(\"czi_registry.db\")")]
	/// path to the database file
	dbfile: PathBuf,
	#[argh(positional)]
	/// czi files to register
	files:Vec<PathBuf>
}
fn main() -> Result<(), Box<dyn Error>> {

	let cli: Cli = argh::from_env();

	let conn = db::init_db(&cli.dbfile)?;

	for fname in cli.files {
		match db::register_file(&conn,&fname){
			RegisterResult::Ok => {}
			RegisterResult::ImageExists(v) =>
				println!("image is already registered, known filenames are {v:?}"),
			RegisterResult::FileExists => println!("{:?} is already registered",fname),
			RegisterResult::SqlErr(e) => eprintln!("{e}"),
			RegisterResult::IoErr(e) => eprintln!("{e}")
		};
	}
	Ok(())
}
