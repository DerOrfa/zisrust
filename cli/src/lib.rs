use std::path::PathBuf;
use std::error::Error;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::Arc;
use db::{DB, RegisterSuccess};
use zisraw::ZisrawInterface;
use zisraw::utils::XmlUtil;
use prettytable::{row, Table};
use uuid::Uuid;

pub fn register(database: &DB, fname: &PathBuf) -> Result<(), Box<dyn Error>> {
	match database.register_file(&fname)? {
		RegisterSuccess::Inserted => println!("{} is now registered", fname.to_string_lossy()),
		RegisterSuccess::ImageExists(v) => println!("image in {} is already registered, known filenames are {v:?}", fname.to_string_lossy()),
		RegisterSuccess::FileExists => println!("{} is already registered", fname.to_string_lossy())
	}
	Ok(())
}

pub fn query(database: DB, where_clause: Option<String>, json:bool) -> Result<(), Box<dyn Error>> {
	let images = database.query_images(where_clause)?;
	if images.is_empty() { return Ok(()) }

	if json {
		println!("{}", serde_json::to_string_pretty(&images)?);
	} else {
		let mut table = Table::new();
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
	Ok(())
}

pub fn dump(name:PathBuf, xmlfile:Option<PathBuf>) -> Result<(), Box<dyn Error>> {
	let file:Arc<dyn FileExt> = Arc::new(File::open(name)?);
	let hd = zisraw::get_file_header(&file)?;
	println!("{hd:#?}");

	let mut metadata = hd.get_metadata(&file)?;

	if let Some(xmlfile) = &xmlfile{
		println!("writing {} bytes xml data to {}",metadata.cache.source.len(),xmlfile.to_string_lossy());
		std::fs::write(xmlfile,metadata.cache.source.clone())?;
	}

	let metadata_tree = metadata.as_tree();

	if let Ok(metadata_tree) = metadata_tree {
		let mut metadata_tree = metadata_tree;
		let image_branch = metadata_tree
			.take_child("Information").unwrap()
			.take_child("Image").unwrap();
		let acquisition_timestamp: chrono::DateTime<chrono::Local> =
			image_branch.child_into("AcquisitionDateAndTime")?;
		println!("acquisition time: {acquisition_timestamp}");

		let found_filename = metadata_tree
				.drill_down(&["Experiment", "ImageName"])?
				.get_text();
		match found_filename {
			None => return Err(std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"failed get original filename from metadata"
			).into()),
			Some(f) => println!("original filename: {f}")
		}
	} else {
		println!("{}",metadata_tree.unwrap_err());
		if xmlfile.is_none() {
			println!("maybe use --xmlfile to write {} bytes to file", metadata.cache.source.len())
		}
	}

	Ok(())
}
