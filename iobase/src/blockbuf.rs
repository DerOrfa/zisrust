use std::os::unix::fs::FileExt;
use std::sync::Arc;
use crate::{DataFromFile,Endian,Result};
use std::mem::size_of;
use std::fmt::{Debug, Formatter};
use std::vec::Drain;
use bytemuck::Pod;
use crate::Error::Own;
use crate::Error;
use crate::basic::ByteSwapper;

pub struct BlockBuf{
	source:Arc<dyn FileExt>,
	start_in_file:u64,
	drained:usize,
	buffer:Vec<u8>,
	endianess:Endian
}

impl BlockBuf {
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
	pub fn drain(&mut self,size:usize) -> Drain<u8>{
		self.drained +=size;
		self.buffer.drain(..size)
	}
	/// Create a new buffer from a shared file.
	///
	/// - the buffer starts at pos in the source file
	/// - endianess describes the endianess of the file
	pub fn new(	source:Arc<dyn FileExt>, pos:u64, endianess: Endian) -> std::io::Result<Self>{
		let mut me=Self{source, start_in_file:pos, drained:0, endianess, buffer:vec![]};
		me.resize(1024)?;//initialize buffer at pos with 1k for now
		Ok(me)
	}
	/// Grows or shrinks the buffer and reads data from the source file if necessary.
	///
	/// **Notice that newsize ignores already drained data.**
	/// So, if you drain 512 bytes from a 1k buffer and than grow it to 2k, you'll end up with 1.5k!
	pub fn resize(&mut self,newsize:usize) -> std::io::Result<()>{
		let oldsize=self.buffer.len();//save old length for later
		let newsize = newsize-self.drained;
		self.buffer.resize(newsize,0);
		if newsize > oldsize { // if buffer has been grown, fill new bytes accordingly
			let target = &mut self.buffer[oldsize..];
			self.source.read_exact_at(target,self.start_in_file+(self.drained+oldsize) as u64)
		} else { Ok(()) }
	}
	/// skips to a specific position by draining the necessary amount of data
	///
	/// - newpos is meant from the beginning of the buffer (aka the position it was originally created at)
	/// - trying to skip to a position that was already drained will return an error and has no other effect
	pub fn skip_to(&mut self,newpos:u64) -> Result<&mut BlockBuf>{
		if newpos < self.drained as u64{
			Err(Error::from("Cannot skip backwards"))
		} else {
			self.drain((newpos as usize - self.drained) as usize);
			Ok(self)
		}
	}
	/// splices of a block buffer with the given size that starts at the current position
	///
	/// - drains all bytes from the this buffer, you'll have to take them back
	/// - clones the source file object
	fn splice_unlimited(&mut self) -> BlockBuf{
		let mut buffer:Vec<u8> = vec![];
		buffer.append(&mut self.buffer);
		BlockBuf{
			source: self.source.clone(),
			start_in_file: self.start_in_file+self.drained as u64,
			drained: 0,
			buffer,
			endianess: self.endianess.clone()
		}
	}
	/// splices of a block buffer with the given size that starts at the current position
	///
	/// - drains size bytes from the this buffer
	/// - clones the source file object
	pub fn splice(&mut self, size:usize) -> BlockBuf{
		BlockBuf{
			source: self.source.clone(),
			start_in_file: self.start_in_file+self.drained as u64,
			drained: 0,
			buffer: self.drain(size).collect(),
			endianess: self.endianess.clone()
		}
	}
	/// splices of a block buffer with the remaining data from this buffer
	///
	/// - invalidates this buffer
	/// - equivalent to resetting the starting point of this buffer to the current position
	pub fn splice_all(self) -> BlockBuf{
		BlockBuf{
			start_in_file: self.start_in_file+self.drained as u64,
			drained: 0,
			..self
		}
	}
	/// Creates a DataFromFiles at the current position with the given size.
	///
	/// - drains size bytes from the buffer.
	pub fn get_cached_data(&mut self,size:usize) -> DataFromFile{
		let ret= DataFromFile::new(&self.source,self.start_in_file+self.drained as u64,size);
		self.drain(size);
		ret
	}
	/// Get a scalar value from the buffer.
	///
	/// - drains size_of::<T>() bytes from the buffer.
	/// - will convert endianess if necessary
	pub fn get_scalar<T:bytemuck::AnyBitPattern+ByteSwapper>(&mut self)->T{
		let ret:T = bytemuck::from_bytes::<T>(
			self.drain(size_of::<T>()).as_slice()
		).clone();
		self.swap_bytes_if_needed(ret)
	}
	/// Get an array of scalar values from the buffer.
	///
	/// - drains N * size_of::<T>() bytes from the buffer.
	/// - will convert endianess if necessary
	pub fn get_array<const N:usize,T:bytemuck::AnyBitPattern+ByteSwapper>(&mut self)->[T;N]{
		std::array::from_fn(|_|self.get_scalar())
	}
	/// Get an vector of scalar values from the buffer.
	///
	/// - drains len * size_of::<T>() bytes from the buffer.
	/// - will convert endianess if necessary
	pub fn get_vec<T:ByteSwapper+Pod>(&mut self,len:usize) -> Vec<T>{
		std::iter::from_fn(||Some(self.get_scalar()))
			.take(len).collect()
	}
	/// Drain given amount of bytes and try to interpret them as string.
	///
	/// - always drains len bytes from the buffer.
	pub fn get_utf8(&mut self, len:usize) -> Result<String>{
		let bytes:Vec<u8> = self.drain(len).collect();
		String::from_utf8(bytes)
			.or_else(|e|Err(Own(format!("Failed to read {len} bytes as utf8-string ({})",e))))
	}
	/// Drain given amount of bytes and try to interpret them as cstring.
	///
	/// - always drains LEN bytes from the buffer even if null.
	/// - the returned string will stop at the the first encountered null-terminator if there is any.
	pub fn get_ascii<const LEN: usize>(&mut self) -> String {
		let drain=self.drain(LEN);
		String::from_utf8_lossy(drain.as_slice())
			.trim_end_matches('\0')
			.to_string()
	}
	/// create an object by reading data from the buffer
	///
	/// Drains some data from the the buffer.
	///
	/// T::read is run with a spliced off buffer.
	/// Because of that T::read will see a buffer with no drained bytes that starts from the current position.
	pub fn read<T,E>(&mut self) -> std::result::Result<T,E> where E:std::error::Error, T:BlockRead<E>
	{
		let mut local = self.splice_unlimited();
		let ret=T::read(&mut local);
		self.drained += local.drained;//add locally drained amount to my own
		self.buffer = local.buffer; //get back remaining bytes from the spliced off buffer
		ret
	}
	/// create a vector of objects by reading all remaining data from the buffer
	pub fn read_vec<T,E>(&mut self,len:usize) -> std::result::Result<Vec<T>,E> where E:std::error::Error, T:BlockRead<E>
	{
		std::iter::from_fn(||Some(self.read())).take(len).collect()
	}
}

pub trait BlockRead<E:std::error::Error>{
	fn read(buffer:&mut BlockBuf) -> std::result::Result<Self,E> where Self:Sized;
}

impl Debug for BlockBuf{
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let current = self.start_in_file+self.drained as u64;
		let remaining = self.buffer.len();
		f.debug_struct("BlockBuf")
			.field("starts at",&self.start_in_file)
			.field("drained",&self.drained)
			.field("endianess",&self.endianess)
			.field("current position in file",&current)
			.field("remaining bytes",&remaining)
			.finish()
	}
}
