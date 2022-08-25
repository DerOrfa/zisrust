use std::error::Error;
use std::fs::File;
use std::io::{BufReader};
//use zisrust::io::{Endian, FileGet, FileRead};
use zisrust::io::zisraw::{get_file_header,ZisrawInterface};
use zisrust::pyramid::Pyramid;


fn main() -> Result<(), Box<dyn Error>> {
	let mut file=BufReader::new(
		File::open("/Users/enrico/ownCloud.gwdg/czi_example/2021_12_02__0243.czi")?
	);
	let hd= get_file_header(&mut file)?;
	let info = hd.get_image_info(&mut file)?;
	let mut d= hd.get_directory(&mut file)?;
	println!("{} blocks found", d.Entries.len());

	for s in info.scenes{
		let _p=Pyramid::new(d.take_tiles(1), s.MinificationFactor );
	}

	// let s = Segment::read(&mut file,&Endian::Little);
	// match s.block {
	// 	SegmentBlock::FileHeader(hd) => {
	// 		file.seek(SeekFrom::Start(hd.DirectoryPosition as u64))?;
	// 		Ok(())
	// 	}
	// 	_ => Err(Box::new(std::io::Error::new(ErrorKind::InvalidData,"Unexpected block when looking for header")))
	// }
	Ok(())
}
