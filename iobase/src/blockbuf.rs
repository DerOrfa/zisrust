use std::os::unix::fs::FileExt;
use std::sync::Arc;
use crate::{DataFromFile,Endian};
use std::mem::size_of;
use std::fmt::{Debug, Formatter};
use std::vec::Drain;
use bytemuck::Pod;
use std::io::Result;
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
	/// read at least min bytes from the file and append them onto the buffer
	fn fetch_at_least(&mut self, min:usize) -> Result<usize>{
		let mut request:isize = min as isize; //request could actually go below zero as we're likely to read more than was requested
		let oldsize = self.buffer.len();
		self.buffer.resize(oldsize+min+1024,0); //always ask (and prepare) for more
		let target = &mut self.buffer[oldsize..];

		while request > 0 {
			let already_red = min-request as usize;
			let relative_pos= (self.drained+oldsize+already_red) as u64;
			match self.source.read_at(target,self.start_in_file+relative_pos) {
				Ok(0) => break, // nothing to see here
				Ok(n) => {request-=n as isize;}
				Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => {} // just try again
				Err(e) => return Err(e.into()),
			}
		}
		assert!(request<=0); // we're done reading, request probably dropped below 0
		let actually_red_data= min+(-request)as usize;//actually red plus overshoot
		self.buffer.shrink_to(oldsize+actually_red_data); //resize to requested size plus the overshoot
		Ok(actually_red_data)
	}
	fn skip(&mut self, len:usize){
		self.drained += len;
		if len < self.buffer.len(){
			self.buffer.drain(..len);
		} else {
			self.buffer = vec![];
		}
	}
	pub fn drain(&mut self,size:usize) -> Result<Drain<u8>>{
		if size > self.buffer.len(){ // make sure, we do have the data
			self.fetch_at_least(size)?;
		}
		self.drained +=size;
		Ok(self.buffer.drain(..size))
	}
	/// Create a new buffer from a shared file.
	///
	/// - the buffer starts at pos in the source file
	/// - endianess describes the endianess of the file
	pub fn new(	source:Arc<dyn FileExt>, pos:u64, endianess: Endian) -> Result<Self>{
		Ok(Self{source, start_in_file:pos, drained:0, endianess, buffer:vec![]})
		//the first drain will initialize buffer at pos with at least 1k
	}
	/// Grows or shrinks the buffer and reads data from the source file if necessary.
	///
	/// **Notice that newsize ignores already drained data.**
	/// So, if you drain 512 bytes from a 1k buffer and than grow it to 2k, you'll end up with 1.5k!
	/// But the returned "size" will still be 20k, or probably more, as the read intentionally "overshoots"
	/// **Notice that, while this will read at least the requested data, ist probably going to read more**
	/// failing to read at least the requested data will return an IO error
	pub fn resize(&mut self, newsize:usize) -> std::io::Result<usize>{
		let oldsize=self.buffer.len();//save old length for later
		let newsize = newsize-self.drained;
		if newsize > oldsize { // needs to grow, fill new bytes accordingly
			let red= self.fetch_at_least(newsize-oldsize)?;
			Ok(red+self.drained)
		} else { // we already have everything, maybe can even cut off some bytes at the end
			self.buffer.shrink_to(newsize);
			Ok(newsize+self.drained)
		} // we actually shrunk the buffer
	}
	/// skips to a specific position by draining the necessary amount of data
	///
	/// - newpos is meant from the beginning of the buffer (aka the position it was originally created at)
	/// - trying to skip to a position that was already drained will return an error and has no other effect
	pub fn skip_to(&mut self, newpos:u64) -> Result<&mut BlockBuf>{
		if newpos < self.drained as u64{
			Err(std::io::Error::new(std::io::ErrorKind::Other,"Cannot skip backwards").into())
		} else {
			self.skip(newpos as usize - self.drained);
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
	pub fn splice(&mut self, size:usize) -> Result<BlockBuf>{
		let buffer = self.drain(size)?.collect();
		Ok(BlockBuf{
			source: self.source.clone(),
			start_in_file: self.start_in_file+self.drained as u64,
			drained: 0,
			buffer,
			endianess: self.endianess.clone()
		})
	}
	/// splices of a block buffer with the remaining data from this buffer
	///
	/// - consumes this buffer
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
	/// - this does not actually read any data, and thus cannot fail
	pub fn get_cached_data(&mut self,size:usize) -> DataFromFile{
		let ret= DataFromFile::new(&self.source,self.start_in_file+self.drained as u64,size);
		self.skip(size);
		ret
	}
	/// Get a scalar value from the buffer.
	///
	/// - drains size_of::<T>() bytes from the buffer.
	/// - will convert endianess if necessary
	pub fn get_scalar<T:bytemuck::AnyBitPattern+ByteSwapper>(&mut self)->Result<T>{
		let ret:T = bytemuck::from_bytes::<T>(
			self.drain(size_of::<T>())?.as_slice()
		).clone();
		Ok(self.swap_bytes_if_needed(ret))
	}
	/// Get an array of scalar values from the buffer.
	///
	/// - drains N * size_of::<T>() bytes from the buffer.
	/// - will convert endianess if necessary
	pub fn get_array<const N:usize,T:bytemuck::AnyBitPattern+ByteSwapper>(&mut self)->Result<[T;N]>{
		// make sure buffer is actually big enough so self.get_scalar() won't fail
		self.fetch_at_least(N*size_of::<T>())?;
		Ok(std::array::from_fn(|_|self.get_scalar().unwrap()))
	}
	/// Get an vector of scalar values from the buffer.
	///
	/// - drains len * size_of::<T>() bytes from the buffer.
	/// - will convert endianess if necessary
	pub fn get_vec<T:ByteSwapper+Pod>(&mut self,len:usize) -> Result<Vec<T>>{
		std::iter::from_fn(||Some(self.get_scalar()))
			.take(len).collect()
	}
	/// Drain given amount of bytes and try to interpret them as string.
	///
	/// - always drains len bytes from the buffer.
	pub fn get_utf8(&mut self, len:usize) -> crate::Result<String>{
		let bytes:Vec<u8> = self.drain(len)?.collect();
		String::from_utf8(bytes)
			.or_else(|e|Err(e.into()))
	}
	/// Drain given amount of bytes and try to interpret them as cstring.
	///
	/// - always drains LEN bytes from the buffer even if null.
	/// - the returned string will stop at the the first encountered null-terminator if there is any.
	pub fn get_ascii<const LEN: usize>(&mut self) -> Result<String> {
		self.drain(LEN)
			.map(|drained|String::from_utf8_lossy(drained.as_slice())
				.trim_end_matches('\0')
				.to_string()
			)
	}
	/// create an object by reading data from the buffer
	///
	/// Drains some data from the the buffer.
	///
	/// T::read is run with a spliced off buffer.
	/// Because of that T::read will see a buffer with no drained bytes that starts from the current position.
	pub fn read<T>(&mut self) -> Result<T> where T:BlockRead	{
		let mut local = self.splice_unlimited();
		let ret=T::read(&mut local);
		self.drained += local.drained;//add locally drained amount to my own
		self.buffer = local.buffer; //get back remaining bytes from the spliced off buffer
		ret
	}
	/// create a vector of objects by reading all remaining data from the buffer
	pub fn read_vec<T>(&mut self,len:usize) -> Result<Vec<T>> where T:BlockRead
	{
		std::iter::from_fn(||Some(self.read())).take(len).collect()
	}
}

pub trait BlockRead{
	fn read(buffer:&mut BlockBuf) -> Result<Self> where Self:Sized;
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
