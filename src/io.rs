use std::io::{Read, Seek};

mod basic;
mod zisraw_impl;
pub mod zisraw;

pub enum Endian{Big,Little}

/// The `FileRead` trait allows for complex structures to be "gotten" from implementors of the `FileGet` trait.
pub trait FileRead<T:Read+Seek> {
	fn read(file:&mut T, endianess: &Endian) ->Self where Self:Sized;
}

/// The `FileGet` trait allows for reading complex structures that implement the FileRead trait from a source.
///
/// This trait has a blanked implementation for implementors of the `Read` and `Seek` trait but some
/// FileRead-implementors might be a bit more restrictive.
/// That means not all structures cen be "gotten" from any implementors of the `Read` and `Seek`.
///
/// Please note that each reading operation is done at the current reading position. You might want to do seek before.
pub trait FileGet<T:Read+Seek> {
	fn get<R:FileRead<T>> (&mut self, endianess: &Endian) ->R;
	fn get_utf8(&mut self, len:u64) -> String;
	fn get_ascii<const LEN: usize>(&mut self) -> String {
		self
			.get::<[char;LEN]>(&Endian::Little)//endinaness is irrelevant here
			.iter().filter(|x| **x>'\0')
			.collect::<String>()
	}
	fn get_vec<R:FileRead<T>>(&mut self, len: usize, endianess: &Endian) -> Vec<R> {
		std::iter::from_fn(|| Some(self.get(endianess)))
			.take(len)
			.collect()
	}
}

