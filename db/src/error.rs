use std::fmt::{Debug, Formatter};
use serde::Serialize;
use serde::ser::{Serializer,SerializeStruct};

#[derive(Debug)]
pub enum Error {
	Other(Box<dyn std::error::Error>),
	Because((Box<dyn std::error::Error>,String)),
	Own(String)
}

impl From<&str> for Error{
	fn from(e: &str) -> Self {Error::Own(e.to_string())}
}

impl std::fmt::Display for Error{
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			Error::Own(e) => std::fmt::Display::fmt(e,f),
			Error::Because((reason,own)) => {
				std::fmt::Display::fmt(&Error::Own(own.clone()),f)?;
				std::fmt::Display::fmt(reason,f)
			}
			Error::Other(e) => std::fmt::Display::fmt(e,f)
		}
	}
}

impl std::error::Error for Error {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Error::Own(_) => None,
			Error::Other(e) => e.source(),
			Error::Because((e,_)) => e.source()
		}
	}
}

// impl Serialize for Error {
// 	fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer,
// 	{
// 		let source = std::error::Error::source(self);
// 		let mut s = serializer.serialize_struct("zisraw error", match source {None => 2,Some(_) => 3})?;
// 		match self {
// 			Error::Io(_) => s.serialize_field("type", "io error")?,
// 			Error::Sqlite(_) => s.serialize_field("type", "sql error")?,
// 			Error::Own(_) => s.serialize_field("type", "zisraw error")?
// 		}
// 		s.serialize_field("source", self.to_string().as_str())?;
// 		if source.is_some(){
// 			s.serialize_field("source", source.unwrap().to_string().as_str())?;
// 		}
// 		s.end()
// 	}
// }
//
