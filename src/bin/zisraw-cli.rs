mod cli;

use std::error::Error;
use std::path::PathBuf;
use argh::FromArgs;
use zisrust::db::DB;

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
	/// czi files to "dump"
	file:PathBuf,
	/// an optional file to write the xmal data to
	#[argh(option,short='x')]
	xmlfile:Option<PathBuf>
}

fn main() -> Result<(), Box<dyn Error>> {

	let cli: Cli = argh::from_env();
	let database = DB::new(&cli.dbfile)?;

	match cli.nested {
		Commands::Register(r) => {
			for fname in r.files {
				cli::register(&database, &fname)?;
			}
		}
		Commands::Query(l) => cli::query(database, l.where_clause, l.json)?,
		Commands::Dump(d)  => cli::dump(d.file, d.xmlfile )?
	}
	Ok(())
}
