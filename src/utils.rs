use xmltree::{Element,ElementPredicate};
use std::fmt::Debug;
use std::str::FromStr;
use std::io::{Result,Error,ErrorKind::InvalidData};

pub trait XmlUtil {
	fn into<T>(&self) -> Result<T> where T:FromStr;
	fn child_into<T, P>(&self, name: P) -> Result<T> where T: FromStr, P:Debug+ElementPredicate;
	fn collect_attributed_values<T, P>(&self, name: P, attr:&str) -> Result<std::collections::HashMap<String,T>>
		where T: FromStr, String:PartialEq<P>;
	fn drill_down(&self,children:&[&str]) -> Result<&Element>;
}

impl XmlUtil for Element{
	fn into<T>(&self) -> Result<T> where T:FromStr{
		let text = self
			.get_text()
			.ok_or(Error::new(InvalidData,format!("Failed to read element {} as text",self.name)))?;
		T::from_str(text.as_ref())
			.or(Err(Error::new(
				InvalidData,format!("Failed to parse elements {} text {}",self.name,text)
			)))
	}
	fn child_into<T, P>(&self, name: P) -> Result<T> where T: FromStr, P:Debug+ElementPredicate
	{
		let prep_err=Error::new(InvalidData,format!("Failed to access child {:?} of {} as text",name,self.name));
		let cld= self.get_child(name).ok_or(prep_err)?;
		XmlUtil::into(cld)
	}

	fn collect_attributed_values<T, P>(&self, name: P, attr:&str) -> Result<std::collections::HashMap<String,T>>
		where T: FromStr, String:PartialEq<P>{

		let chld=self.children.iter()
			.filter_map(|e|e.as_element())
			.filter(|e|e.name==name);
		let mut ret:std::collections::HashMap<String,T>=Default::default();
		for e in chld {
			let value:T = e.child_into("Value")?;
			let id=e.attributes.get(attr)
				.ok_or(Error::new(InvalidData,format!("attribute {} missing in {}",attr,e.name)))?;
			ret.insert(id.clone(),value);
		}
		if ret.is_empty(){Err(Error::new(InvalidData,format!("no values found")))}
		else {Ok(ret)}
	}

	fn drill_down(&self, children: &[&str]) -> Result<&Element> {
		let childrens= children.len();
		self.get_child(children[0])
			.ok_or(Error::new(
				InvalidData,
				format!("Failed to walk down the element chain {:?}=>{:?}",children[0],&children[1..])
			))
			.and_then(|child|
				if childrens==1 {Ok(child)}
				else {child.drill_down(&children[1..])}
			)

	}
}
