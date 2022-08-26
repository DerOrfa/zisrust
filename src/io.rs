use std::io::{Read, Result, Seek, BufReader, Error};
use crate::io::basic::Cached;
use std::ffi::CStr;
use std::io::ErrorKind::InvalidInput;

mod basic;
pub mod zisraw;

pub enum Endian{Big,Little}

/// The `FileRead` trait allows for complex structures to be "gotten" from implementors of the `FileGet` trait.
pub trait FileRead<T:Read> {
	fn read(file:&mut T, endianess: &Endian) -> Result<Self> where Self:Sized;
}

/// The `FileGet` trait allows for reading complex structures that implement the FileRead trait from a source.
///
/// This trait has a blanked implementation for implementors of the `Read` and `Seek` trait but some
/// FileRead-implementors might be a bit more restrictive.
/// That means not all structures cen be "gotten" from any implementors of the `Read` and `Seek`.
///
/// Please note that each reading operation is done at the current reading position. You might want to do seek before.
pub trait FileGet<T:Read> {
	fn get<R:FileRead<T>> (&mut self, endianess: &Endian) ->Result<R>;
	fn get_utf8(&mut self, len:u64) -> Result<String>;
	fn get_ascii<const LEN: usize>(&mut self) -> Result<String> {
		let bytes:[u8;LEN]=self.get(&Endian::Little)?;//endinaness is irrelevant here
		// todo replace with CStr::from_bytes_until_nul once its stable
		match unsafe{CStr::from_bytes_with_nul_unchecked(&bytes)}.to_str(){
			Ok(s) => Ok(String::from(s.trim_end_matches('\0'))),
			Err(e) => Err(Error::new(InvalidInput,format!("Failed to read bytes {:?} as utf8-string ({})",bytes,e)))
		}
	}
	fn get_vec<R:FileRead<T>>(&mut self, len: usize, endianess: &Endian) -> Result<Vec<R>> {
		std::iter::from_fn(|| Some(self.get(endianess)))
			.take(len)
			.collect()
	}
}

#[derive(Debug)]
pub struct Data{
	cache: Cached<memmap::Mmap,Vec<u8>>
}
impl Data {
	pub fn new(file:&mut BufReader<std::fs::File>,size:usize)->Result<Data>{
		let pos= file.stream_position()?;
		let mmap = unsafe{
			memmap::MmapOptions::new()
				.offset(pos)
				.len(size)
				.map(file.get_ref())
		}?;
		file.seek_relative(size as i64)?;//simulate consumption of the data
		Ok(Data{
			cache: basic::Cached::new(mmap,Self::produce)
		})
	}
	pub fn get(&mut self)->&Vec<u8>{
		self.cache.get()
	}
	fn produce(source:&memmap::Mmap)->Vec<u8>{
		source.to_vec()
	}
}

