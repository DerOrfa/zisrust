use db::Error;
use std::path::PathBuf;
use argh::FromArgs;
use db::DB;
use db::Error::Own;

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
	Dump(Dump)
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

#[derive(FromArgs, PartialEq, Debug)]
/// print out metadata of a file
#[argh(subcommand, name = "dump")]
struct Dump {
	#[argh(positional)]
	/// czi file to "dump" either in filename or guid
	file:String,
	/// an optional file to write the xml data to
	#[argh(option,short='x')]
	xmlfile:Option<PathBuf>
}

fn main() -> Result<(), Box<dyn std::error::Error>> {

	let cli: Cli = argh::from_env();
	let database = DB::new(&cli.dbfile)?;

	match cli.nested {
		Commands::Register(r) => {
			for fname in r.files {
				cli::register(&database, &fname)?;
			}
		}
		Commands::Query(l) => cli::query(database, l.where_clause, l.json)?,
		Commands::Dump(d)  => {
			let fname = match uuid::Uuid::parse_str(d.file.as_str()).ok(){
				None => d.file.into(),
				Some(uuid) => database
					.query_images(Some(format!("guid = \"{uuid}\"")))?.first()
					.ok_or(Own(format!("Image with guid \"{uuid}\" not found in \"{}\"",cli.dbfile.to_string_lossy())))?
					.filenames.iter()
					.find_map(|f|f.exists().then_some(f))
					.ok_or(Own(format!("None of the files registered with guid \"{uuid}\" could be found or accessed")))?
					.clone()
			};

			cli::dump(fname, d.xmlfile )?
		}
	}
	Ok(())
}
