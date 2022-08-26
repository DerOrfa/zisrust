use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use std::io::{Read, Result};
use std::time::Instant;
use crate::io::FileGet;
use super::{FileRead, Endian};

trait Integer{
	fn swap_bytes(self) -> Self;
}

// impl FromStr for PixelType{
// 	type Err = std::io::Error;
//
// 	fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
// 		Ok(match s {
// 			"Gray8" => Gray8,
// 			"Gray16" => Gray16,
// 			"Gray32" => Gray32,
// 			"Gray64" => Gray64,
// 			"Bgr24" => Bgr24,
// 			"Bgr48" => Bgr48,
// 			"Bgra32" => Bgra32,
// 			"Bgr96Float" => Bgr96Float,
// 			"Gray32Float" => Gray32Float,
// 			"Gray64ComplexFloat" => Gray64ComplexFloat,
// 			"Bgr192ComplexFloat" => Bgr192ComplexFloat,
// 			_ => return Err(std::io::Error::new(
// 				ErrorKind::InvalidData,
// 				"Failed to interpret {s} as a pixeltype"
// 			))
// 		})
// 	}
// }

pub struct Cached<S,T>{
	store:Option<T>,
	pub source:S,
	producer:fn(&S)->T,
	last_use:Instant
}

impl<S,T> Debug for Cached<S,T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let ago = Instant::now()-self.last_use;
		write!(f,"Empty cache last used {ago:?} ago")
	}
}

impl<S,T> Cached<S,T> {
	pub fn new(source:S,producer:fn(&S)->T) -> Cached<S,T>{
		Cached{
			producer, source,
			store:None,
			last_use: std::time::Instant::now()
		}
	}
	pub fn get(&mut self)->&T{
		self.last_use= std::time::Instant::now();
		self.store.get_or_insert_with(||(self.producer)(&self.source))
	}
}

impl<T:Read> FileGet<T> for T{
	fn get<R: FileRead<T>>(&mut self, endianess: &Endian) -> Result<R> {
		R::read(self,endianess)
	}
	fn get_utf8(&mut self, len: u64) -> Result<String> {
		let mut s=String::new();
		self.take(len).read_to_string(&mut s)?;
		Ok(s)
	}
}

impl<T: Read> FileRead<T> for f32 {
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(f32::from_bits(u32::read(file,endianess)?))
	}
}
impl<T: Read> FileRead<T> for f64 {
	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
		Ok(f64::from_bits(u64::read(file,endianess)?))
	}
}

impl<T: Read> FileRead<T> for uuid::Uuid{
	fn read(file: &mut T, _: &Endian) -> Result<Self> {
		let id:[u8;16] = file.get(&Endian::Little)?;
		Ok(uuid::Uuid::from_bytes(id))
	}
}

impl<const N:usize,T: Read> FileRead<T> for [u8;N] {
	fn read(file: &mut T, _: &Endian) -> Result<Self> {
		let mut ret=[0;N];
		file.read_exact(&mut ret)?;
		Ok(ret)
	}
}

impl<I: Integer+Default,T: Read> FileRead<T> for I {
	fn read(file: &mut T, endianess: &Endian) -> Result<Self>{
		let ret:Result<Self>=raw_read(file);
		#[cfg(target_endian = "little")]
		{
			match endianess {
				Endian::Big => Ok(ret?.swap_bytes()),
				Endian::Little => ret
			}
		}
		#[cfg(not(target_endian = "little"))]
		{
			match endianess {
				Endian::Big => ret,
				Endian::Little => ret.swap_bytes()
			}
		}
	}
}
impl Integer for u16{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl Integer for i16{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl Integer for u32{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl Integer for i32{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl Integer for u64{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl Integer for i64{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl Integer for u128{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl Integer for i128{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}

fn raw_read<T: Read,R: Default+Integer>(file: &mut T) -> Result<R> {
	let mut ret: R = R::default();

	// scary pointer trickery
	let ptr: *mut R = &mut ret;
	let buff:&mut [u8]=
		unsafe {std::slice::from_raw_parts_mut(ptr as *mut u8, size_of::<R>())};
	// ok buff should occupy exactly the same mem as ret, so loading into it, should load into ret
	file.read_exact(buff)?;
	Ok(ret)
}
