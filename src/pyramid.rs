use std::cmp::Ordering;
use euclid::Rect;
use num_complex::Complex;
use ndarray::Array2;

pub struct PixelSpace;
pub struct RealSpace;

pub trait Tile{
	fn frame(&self) -> Rect<i32, PixelSpace>;
	fn pixel(&self)->Pixel;
	fn level(&self, scaling:i32)->usize;
	fn ordering_id(&self) -> i32;
}

impl PartialEq for dyn Tile {
	fn eq(&self, other: &Self) -> bool {
		self.partial_cmp(other).unwrap().is_eq()
	}
}

impl PartialOrd for dyn Tile{
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		self.ordering_id().partial_cmp(&other.ordering_id())
	}
}

#[derive(Debug)]
pub enum Pixel{
	Gray8(Array2<u8>),
	Gray16(Array2<u16>),
	Gray32(Array2<u32>),
	Gray64(Array2<u64>),
	Bgr24(Array2<(u8,u8,u8)>),
	Bgr48(Array2<(u16,u16,u16)>),
	Bgra32(Array2<(u8,u8,u8,u8)>),
	Bgr96Float(Array2<(f32,f32,f32)>),
	Gray32Float(Array2<f32>),
	Gray64ComplexFloat(Array2<Complex<f32>>),
	Bgr192ComplexFloat(Array2<(Complex<f32>,Complex<f32>,Complex<f32>)>)
}


pub struct Pyramid{
	layers:Vec<Vec<Box<dyn Tile>>>
}

impl Pyramid{
	pub fn new(tiles:Vec<Box<dyn Tile>>, scaling_factor:i32) -> Self{
		let mut ret=Pyramid{layers:vec![]};
		for tile in tiles{
			ret.layers[tile.level(scaling_factor)].push(tile);
		}
		ret
	}
}
