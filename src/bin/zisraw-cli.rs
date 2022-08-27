use std::error::Error;
use std::path::PathBuf;
use argh::FromArgs;
use zisrust::db::{DB, RegisterSuccess};

#[macro_use] extern crate prettytable;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(description = "sqlite backed registry for czi files")]
/// Top-level command.
struct Cli {
	#[argh(subcommand)]
	nested: Commands,
	#[argh(option, short='d', default = "PathBuf::from(\"czi_registry.db\")")]
	/// path to the database file
	dbfile: PathBuf,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Commands {
	Register(Register),
	Query(Query),
}

#[derive(FromArgs, PartialEq, Debug)]
/// register czi images
#[argh(subcommand, name = "register")]
struct Register {
	#[argh(positional)]
	/// czi files to register
	files:Vec<PathBuf>
}

#[derive(FromArgs, PartialEq, Debug)]
/// list registered images
#[argh(subcommand, name = "query")]
struct Query {
	/// an optional "WHERE" clause
	#[argh(positional)]
	where_clause:Option<String>,
	/// output as json instead of table
	#[argh(switch, short = 'j')]
	json:bool
}

fn main() -> Result<(), Box<dyn Error>> {

	let cli: Cli = argh::from_env();

	let database = DB::new(&cli.dbfile)?;

	match cli.nested {
		Commands::Register(r) => {
			for fname in r.files {
				match database.register_file(&fname)?{
					RegisterSuccess::Inserted => {}
					RegisterSuccess::ImageExists(v) => println!("image in {} is already registered, known filenames are {v:?}",fname.to_string_lossy()),
					RegisterSuccess::FileExists => println!("{} is already registered",fname.to_string_lossy())
				};
			}
		}
		Commands::Query(l) => {
			let images=database.query_images(l.where_clause)?;

			if images.is_empty(){return Ok(())}
			if l.json{
				println!("{}",serde_json::to_string_pretty(&images)?);
			} else {
				let mut table = prettytable::Table::new();
				table.add_row(row!["acquisition time","guid","parents guid","original path","file part","known files"]);
				for r in database.query_images(None)? {
					table.add_row(row![
						r.timestamp.to_string(),
						r.guid.to_string(),
						r.parent_guid.map_or("None".to_string(),|g|g.to_string()),
						r.orig_path.to_string_lossy().to_string(),
						r.file_part.to_string(),
						format!("{} copies",r.filenames.len())
					]);
				}
				table.printstd();
			}
		}
	}
	Ok(())
}
