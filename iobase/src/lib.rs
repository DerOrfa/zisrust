use std::error;
use std::fmt::Formatter;
use basic::Cached;
use std::os::unix::fs::FileExt;
use std::sync::Arc;

pub mod basic;
pub mod blockbuf;

#[derive(Debug,Clone)]
pub enum Endian{Big,Little}

pub type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(Debug)]
pub struct DataFromFile{
    cache: Cached<(Arc<dyn FileExt>,u64,usize),Vec<u8>>
}
impl DataFromFile {
    pub fn new(file:&Arc<dyn FileExt>, pos:u64,size:usize)->Self{
        Self{
            cache: Cached::new((file.clone(),pos,size),Self::produce)
        }
    }
    pub fn get(&mut self)->Result<&Vec<u8>>{self.cache.get()}
    fn produce(source:&(Arc<dyn FileExt>,u64,usize))->Result<Vec<u8>>{
        let mut buff = vec![0;source.2];
        source.0.read_exact_at(buff.as_mut_slice(),source.1)?;
        Ok(buff)
    }
}


