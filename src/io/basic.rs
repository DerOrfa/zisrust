use std::fmt::{Debug, Formatter};
use std::mem::size_of;
use std::io::{Read,Seek};
use std::time::Instant;
use crate::io::FileGet;
use super::{FileRead, Endian};

trait Integer{
	fn swap_bytes(self) -> Self where Self:Sized;
}

pub struct Cached<S,T>{
	store:Option<T>,
	pub source:S,
	producer:fn(&S)->T,
	last_use:Instant
}

impl<S,T> Debug for Cached<S,T>{
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let ago = Instant::now()-self.last_use;
		write!(f,"Empty cache last used {ago:?} ago")
	}
}

impl<S,T> Cached<S,T>{
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

impl<T:Read+Seek> FileGet<T> for T{
	fn get<R: FileRead<T>>(&mut self, endianess: &Endian) -> R {
		R::read(self,endianess)
	}
	fn get_utf8(&mut self, len: u64) -> String {
		let mut s=String::new();
		self.take(len).read_to_string(&mut s).expect("Failed reading utf8 text");
		s
	}
}

impl<T: Read+Seek> FileRead<T> for f32 {
	fn read(file: &mut T, endianess: &Endian) -> Self {
		f32::from_bits(u32::read(file,endianess))
	}
}
impl<T: Read+Seek> FileRead<T> for f64 {
	fn read(file: &mut T, endianess: &Endian) -> Self {
		f64::from_bits(u64::read(file,endianess))
	}
}

impl<T: Read+Seek> FileRead<T> for uuid::Uuid{
	fn read(file: &mut T, _: &Endian) -> Self {
		let mut id = [0;16];
		file.read_exact(&mut id).expect("Failed to read");
		uuid::Uuid::from_bytes(id)
	}
}

impl<const N:usize,T: Read+Seek> FileRead<T> for [u8;N] {
	fn read(file: &mut T, _: &Endian) -> Self where Self: Sized {
		let mut ret=[0;N];
		file.read_exact(&mut ret).expect("Failed to read");
		ret
	}
}

impl<const N:usize,T: Read+Seek> FileRead<T> for [char;N]{
	fn read(file: &mut T, _: &Endian) -> [char;N] {
		let buff:[u8;N]=file.get(&Endian::Little);
		let mut ret=['\0';N];
		for i in 0..N{
			ret[i]=buff[i].into();//no utf8 checking necessary as the lower 8bits we can possibly get are safe
		}
		ret
	}
}

impl<I: Integer+Default,T: Read+Seek> FileRead<T> for I {
	fn read(file: &mut T, endianess: &Endian) -> Self where Self: Sized{
		let ret:Self=raw_read(file);
		#[cfg(target_endian = "little")]
		{
			match endianess {
				Endian::Big => ret.swap_bytes(),
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
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}
impl Integer for i16{
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}
impl Integer for u32{
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}
impl Integer for i32{
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}
impl Integer for u64{
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}
impl Integer for i64{
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}
impl Integer for u128{
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}
impl Integer for i128{
	fn swap_bytes(self) -> Self where Self: Sized {self.swap_bytes()}
}

fn raw_read<T: Read+Seek,R: Default+Integer>(file: &mut T) -> R {
	let mut ret: R = R::default();

	// scary pointer trickery
	let ptr: *mut R = &mut ret;
	let buff:&mut [u8]=
		unsafe {std::slice::from_raw_parts_mut(ptr as *mut u8, size_of::<R>())};
	// ok buff should occupy exactly the same mem as ret, so loading into it, should load into ret
	file.read_exact(buff).expect("Failed to read file");
	ret
}
