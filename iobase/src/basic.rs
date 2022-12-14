use std::f64;
use std::fmt::{Debug, Formatter};
use std::time::Instant;
use crate::Result;

pub trait ByteSwapper {
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
	producer:fn(&S)->Result<T>,
	last_use:Instant
}

impl<S,T> Debug for Cached<S,T> {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let ago = Instant::now()-self.last_use;
		write!(f,"Empty cache last used {ago:?} ago")
	}
}

impl<S,T> Cached<S,T> {
	pub fn new(source:S,producer:fn(&S)->Result<T>) -> Cached<S,T>{
		Cached{
			producer, source,
			store:None,
			last_use: Instant::now()
		}
	}
	pub fn get(&mut self)->Result<&T>{
		self.last_use= Instant::now();
		if self.store.is_none() { // try to produce
			let prod= (self.producer)(&self.source)?;
			Ok(self.store.insert(prod))
		} else {
			Ok(self.store.as_ref().unwrap())
		}
	}
}

// impl<T:Read> FileGet<T> for T{
// 	fn get_scalar<R: FileRead<T>>(&mut self, endianess: &Endian) -> Result<R> {
// 		R::read(self,endianess)
// 	}
// 	fn get_utf8(&mut self, len: u64) -> Result<String> {
// 		let mut s=String::new();
// 		let mut reader = self.take(len);
// 		let mut writer = File::create("/tmp/dump.xml")?;
// 		let copied=std::io::copy(&mut reader,&mut writer);
// 		// let red = reader.read_to_string(&mut s);
// 		todo!()
// 	}
// }
//
// impl<T: Read> FileRead<T> for f32 {
// 	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
// 		Ok(f32::from_bits(u32::read(file,endianess)?))
// 	}
// }
// impl<T: Read> FileRead<T> for f64 {
// 	fn read(file: &mut T, endianess: &Endian) -> Result<Self> {
// 		Ok(f64::from_bits(u64::read(file,endianess)?))
// 	}
// }
//
// impl<T: Read> FileRead<T> for uuid::Uuid{
// 	fn read(file: &mut T, _: &Endian) -> Result<Self> {
// 		let id:[u8;16] = file.get_scalar(&Endian::Little)?;
// 		Ok(uuid::Uuid::from_bytes(id))
// 	}
// }

// impl<const N:usize,T: Read> FileRead<T> for [u8;N] {
// 	fn read(file: &mut T, _: &Endian) -> Result<Self> {
// 		let mut ret=[0;N];
// 		file.read_exact(&mut ret)?;
// 		Ok(ret)
// 	}
// }

// impl<I: ByteSwapper +Default,T: Read> FileRead<T> for I {
// 	fn read(file: &mut T, endianess: &Endian) -> Result<Self>{
// 		let ret:Result<Self>=raw_read(file);
// 		#[cfg(target_endian = "little")]
// 		{
// 			match endianess {
// 				Endian::Big => Ok(ret?.swap_bytes()),
// 				Endian::Little => ret
// 			}
// 		}
// 		#[cfg(not(target_endian = "little"))]
// 		{
// 			match endianess {
// 				Endian::Big => ret,
// 				Endian::Little => ret.swap_bytes()
// 			}
// 		}
// 	}
// }

// no-ops for bytes
impl ByteSwapper for u8 {fn swap_bytes(self) -> Self {self}}
impl ByteSwapper for i8 {fn swap_bytes(self) -> Self {self}}

impl ByteSwapper for u16{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl ByteSwapper for i16{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl ByteSwapper for u32{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl ByteSwapper for i32{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl ByteSwapper for u64{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl ByteSwapper for i64{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl ByteSwapper for u128{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}
impl ByteSwapper for i128{
	fn swap_bytes(self) -> Self {self.swap_bytes()}
}

impl ByteSwapper for f32{
	fn swap_bytes(self) -> Self {
		Self::from_bits(self.to_bits().swap_bytes())
	}
}
impl ByteSwapper for f64{
	fn swap_bytes(self) -> Self {
		Self::from_bits(self.to_bits().swap_bytes())
	}
}
//
// fn raw_read<T: Read,R: Default+ ByteSwapper>(file: &mut T) -> Result<R> {
// 	let mut ret: R = R::default();
//
// 	// scary pointer trickery
// 	let ptr: *mut R = &mut ret;
// 	let buff:&mut [u8]=
// 		unsafe {std::slice::from_raw_parts_mut(ptr as *mut u8, size_of::<R>())};
// 	// ok buff should occupy exactly the same mem as ret, so loading into it, should load into ret
// 	file.read_exact(buff)?;
// 	Ok(ret)
// }
