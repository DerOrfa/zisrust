use std::error::Error;
use std::path::PathBuf;
use argh::FromArgs;
use zisrust::db::{DB,RegisterResult, RegisterSuccess};

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

	let database = DB::new(&cli.dbfile)?;

	for fname in cli.files {
		match database.register_file(&fname)?{
			RegisterSuccess::Inserted => {}
			RegisterSuccess::ImageExists(v) => println!("image is already registered, known filenames are {v:?}"),
			RegisterSuccess::FileExists => println!("{:?} is already registered",fname)
		};
	}
	Ok(())
}
