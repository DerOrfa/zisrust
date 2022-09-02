#![allow(non_snake_case)]

use std::borrow::Borrow;
use std::collections::HashMap;
use std::iter::Iterator;
use std::os::unix::fs::FileExt;
use std::sync::Arc;
use uom::si::{f64::Length,length::meter};
use iobase::{Error, Result};

pub mod structs;
pub mod utils;
mod segment;

use utils::XmlUtil;

pub fn get_file_header(file:&Arc<dyn FileExt>) -> Result<structs::FileHeader>{
	let s = segment::Segment::new(file, 0)?;
	match s.block {
		segment::SegmentBlock::FileHeader(hd) => Ok(hd),
		_ => Err(Error::from("Unexpected block when looking for header"))
	}
}

#[derive(Debug)]
pub struct Scene{
	pub RegionId:String,
	pub PyramidLayersCount:usize,
	pub MinificationFactor:i32
}

#[derive(Debug)]
pub struct ImageInfo{
	pub pixels:(u64,u64,u64),
	pub pixel_size:HashMap<String,Length>,
	pub pixel_type:String,
	pub acquisition_timestamp: Option<chrono::DateTime<chrono::Local>>,
	pub acquisition_duration: Option<std::time::Duration>,
	pub mosaic_tiles:Option<u64>,
	pub scenes:Vec<Scene>
}

pub trait ZisrawInterface{
	fn get_metadata(&self,file:&Arc<dyn FileExt>) -> Result<structs::Metadata>;
	fn get_directory(&self,file:&Arc<dyn FileExt>) -> Result<structs::Directory>;
	fn get_attachments(&self,file:&Arc<dyn FileExt>)-> Result<Vec<structs::AttachmentEntryA1>>;

	fn get_metadata_xml(&self,file:&Arc<dyn FileExt>) -> Result<String>{
		let e = self.get_metadata(file)?;
		Ok(e.cache.source.clone())
	}
	fn get_image_info(&self,file:&Arc<dyn FileExt>) -> Result<ImageInfo>{
		let scaling_path=["Scaling","Items"];
		let mut meta = self.get_metadata(file)?.as_tree()?;
		let image_props = meta
			.take_child("Information").unwrap()
			.take_child("Image").unwrap();
		let scaling_el = meta
			.drill_down(&scaling_path)
			.or(image_props.drill_down(&scaling_path));

		let scenes = image_props.drill_down(["Dimensions","S","Scenes"].borrow()).ok();
		let mut info = ImageInfo{
			pixels:(
				image_props.child_into("SizeX")?,
				image_props.child_into("SizeY")?,
				image_props.child_into("SizeZ")?
			),
			pixel_size: Default::default(),
			pixel_type: image_props.child_into("PixelType")?,
			acquisition_timestamp: image_props.child_into("AcquisitionDateAndTime").ok(),
			acquisition_duration: image_props.child_into("AcquisitionDuration")
				.and_then(|d|Ok(std::time::Duration::from_secs_f32(d))).ok(),
			mosaic_tiles: image_props.child_into("SizeM").ok(),
			scenes:vec![]
		};

		if scaling_el.is_ok(){
			let scaling_el= scaling_el.unwrap()
				.collect_attributed_values("Distance","Id")
				.unwrap_or_default()
				.into_iter().map(|(k,v)|(k.to_ascii_lowercase(),Length::new::<meter>(v)));
			info.pixel_size=scaling_el.collect();
		}

		if scenes.is_some() { // no scenes => no pyramid => flat image
			let scenes = scenes.unwrap().children.iter().filter_map(|n|n.as_element());
			for e in scenes{
				let pinfo=e.drill_down(["PyramidInfo"].borrow())?;
				info.scenes.push(Scene{
					RegionId: e.child_into("RegionId")?,
					PyramidLayersCount: pinfo.child_into("PyramidLayersCount")?,
					MinificationFactor: pinfo.child_into("MinificationFactor")?
				});
			}
		}
		Ok(info)
	}
	fn get_thumbnail(&self, file:&Arc<dyn FileExt>) -> Result<Option<structs::Attachment>>{
		let thumbnail = self.get_attachments(file)?
			.into_iter()
			.filter(|a|a.Name=="Thumbnail")
			.next();

		if thumbnail.is_some(){
			let att = segment::Segment::new(file,thumbnail.unwrap().FilePosition)?;
			let att= match att.block{
				segment::SegmentBlock::Attachment(a) => a,
				_ => return Err(Error::from("Unexpected block when looking for attachment"))
			};
			Ok(Some(att))
		} else {Ok(None)}



	}
}
