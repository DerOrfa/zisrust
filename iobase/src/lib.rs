use std::fmt::Formatter;
use basic::Cached;
use std::os::unix::fs::FileExt;
use std::sync::Arc;

pub mod basic;
pub mod blockbuf;

#[derive(Debug,Clone)]
pub enum Endian{Big,Little}

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Own(String)
}
pub type Result<T> = std::result::Result<T, Error>;

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

impl From<std::io::Error> for Error{
    fn from(e: std::io::Error) -> Self {Error::Io(e)}
}
impl From<&str> for Error{
    fn from(e: &str) -> Self {Error::Own(e.to_string())}
}

impl std::fmt::Display for Error{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(e) => std::fmt::Display::fmt(e,f),
            Error::Own(e) => std::fmt::Display::fmt(e,f)
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(e) => e.source(),
            Error::Own(_) => None
        }
    }
}

