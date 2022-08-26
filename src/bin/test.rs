use std::error::Error;
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::sync::Arc;
use zisrust::io::zisraw::{get_file_header,ZisrawInterface};
use zisrust::pyramid::Pyramid;


fn main() -> Result<(), Box<dyn Error>> {
	let file:Arc<dyn FileExt>=	Arc::new(File::open("/media/enrico/5eea2c8b-6974-4121-8d45-97977221fe04/ownCloud.gwdg/2018_10_05__0018_pt1.czi")?);
	let hd= get_file_header(&file)?;
	let info = hd.get_image_info(&file)?;
	let mut d= hd.get_directory(&file)?;
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
