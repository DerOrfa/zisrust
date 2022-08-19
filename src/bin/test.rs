use std::error::Error;
use std::fs::File;
use std::io::{BufReader};
//use zisrust::io::{Endian, FileGet, FileRead};
use zisrust::io::zisraw;


fn main() -> Result<(), Box<dyn Error>> {
	let mut file=BufReader::new(
		File::open("/Users/enrico/ownCloud.gwdg/czi_example/2021_12_02__0243.czi")?
	);
	let hd= zisraw::get_file_header(&mut file)?;
	let d= hd.get_directory(&mut file)?;
//	eprintln!("{:#?}",d.Entries);
	println!("{} blocks found", d.Entries.len());
	println!("{}",hd.get_metadata_xml(&mut file)?);
	// let s = Segment::read(&mut file,&Endian::Little);
	// match s.block {
	// 	SegmentBlock::FileHeader(hd) => {
	// 		file.seek(SeekFrom::Start(hd.DirectoryPosition as u64))?;
	// 		Ok(())
	// 	}
	// 	_ => Err(Box::new(std::io::Error::new(ErrorKind::InvalidInput,"Unexpected block when looking for header")))
	// }
	Ok(())
}
