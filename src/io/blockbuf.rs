use std::os::unix::fs::FileExt;
use std::sync::Arc;
use crate::io::DataFromFile;
use crate::Result;
use crate::io::Endian;
use std::mem::size_of;
use std::ffi::CStr;
use std::vec::Drain;
use bytemuck::Pod;
use crate::Error::Own;
use crate::Error;
use crate::io::basic::ByteSwapper;

pub struct BlockBuf{
	source:Arc<dyn FileExt>,
	start_in_file:u64,
	drained:usize,
	buffer:Vec<u8>,
	endianess:Endian
}

impl BlockBuf {
	pub fn drain(&mut self,size:usize) -> Drain<u8>{
		self.drained +=size;
		self.buffer.drain(..size)
	}
	fn swap_bytes_if_needed<T:ByteSwapper>(&self,t:T)->T{
		#[cfg(target_endian = "little")]
		{
			match self.endianess {
				Endian::Big => t.swap_bytes(),
				Endian::Little => t
			}
		}
		#[cfg(not(target_endian = "little"))]
		{
			match endianess {
				Endian::Big => t,
				Endian::Little => t.swap_bytes()
			}
		}

	}
	pub fn new(	source:Arc<dyn FileExt>, pos:u64, endianess: Endian) -> std::io::Result<Self>{
		let mut me=Self{source, start_in_file:pos, drained:0, endianess, buffer:vec![]};
		me.resize(1024)?;//initialize buffer at pos with 1k for now
		Ok(me)
	}
	/// Grows or shrinks the buffer an reads data from the source file if necessary.
	///
	/// # Noice: newsize ignores already drained data.
	/// So, if you drain 512 bytes from a 1k buffer and than grow it to 2k, you'll end up with 1.5k!
	pub fn resize(&mut self,newsize:usize) -> std::io::Result<()>{
		let oldsize=self.buffer.len();//save old length for later
		let newsize = newsize-self.drained;
		self.buffer.resize(newsize,0);
		if newsize > oldsize{ // if buffer has been grown, fill new bytes accordingly
			let target = &mut self.buffer[oldsize..];
			self.source.read_exact_at(target,self.start_in_file+(self.drained+oldsize) as u64)
		} else { Ok(()) }
	}
	pub fn skip_to(&mut self,newpos:u64) -> Result<()>{
		if newpos < self.drained as u64{
			Err(Error::from("Cannot seek back"))
		} else {
			self.drain((newpos as usize - self.drained) as usize);
			Ok(())
		}
	}
	pub fn get_cached_data(&mut self,size:usize) -> DataFromFile{
		let ret= DataFromFile::new(&self.source,self.drained as u64,size);
		self.drain(size);
		ret
	}
	pub fn get_scalar<T:bytemuck::AnyBitPattern+ByteSwapper>(&mut self)->T{
		let ret:T = bytemuck::from_bytes::<T>(
			self.drain(size_of::<T>()).as_slice()
		).clone();
		self.swap_bytes_if_needed(ret)
	}
	pub fn get_array<const N:usize,T:bytemuck::AnyBitPattern+ByteSwapper>(&mut self)->[T;N]{
		std::array::from_fn(|_|self.get_scalar())
	}
	pub fn get_utf8(&mut self, len:usize) -> Result<String>{
		let bytes:Vec<u8> = self.drain(len).collect();
		String::from_utf8(bytes)
			.or_else(|e|Err(Own(format!("Failed to read {len} bytes as utf8-string ({})",e))))
	}
	pub fn get_ascii<const LEN: usize>(&mut self) -> Result<String> {
		let drain=self.drain(LEN);
		// todo replace with CStr::from_bytes_until_nul once its stable
		match unsafe{CStr::from_bytes_with_nul_unchecked(drain.as_slice())}.to_str(){
			Ok(s) => Ok(String::from(s.trim_end_matches('\0'))),
			Err(e) => Err(Own(format!("Failed to read bytes {LEN} as utf8-string ({})",e)))
		}
	}
	pub fn get_vec<T:ByteSwapper+Pod>(&mut self,len:usize) -> Vec<T>{
		std::iter::from_fn(||Some(self.get_scalar()))
			.take(len).collect()
	}
}
