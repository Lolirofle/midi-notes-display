#![allow(dead_code)]

use core::{mem,ptr};

///Iterator of two or less elements by move.
///Useful because [_,_].into_iter() is an iterator by ref. A generic array cannot store the iteration index.
///
///∀(i∊(0..2)). (i<offset) → (data[i] is invalid)
pub struct PairIter<T>{
	offset: u8,
	data: [T; 2],
}
impl<T> PairIter<T>{
	pub fn get(&self,i: usize) -> Option<&T>{
		if i < (2-self.offset) as usize{
			Some(unsafe{self.data.get_unchecked((self.offset as usize)+i)})
		}else{
			None
		}
	}

	pub fn get_mut(&mut self,i: usize) -> Option<&mut T>{
		if i < (2-self.offset) as usize{
			Some(unsafe{self.data.get_unchecked_mut((self.offset as usize)+i)})
		}else{
			None
		}
	}
}
impl<T> From<[T; 0]> for PairIter<T>{
	fn from(_: [T; 0]) -> Self{PairIter{
		offset: 2,
		data: unsafe{mem::uninitialized()},
	}}
}
impl<T> From<[T; 1]> for PairIter<T>{
	fn from(array: [T; 1]) -> Self{PairIter{
		offset: 1,
		data: [
			unsafe{mem::uninitialized()},
			unsafe{ptr::read(array.get_unchecked(0) as *const T)},
		],
	}}
}
impl<T> From<[T; 2]> for PairIter<T>{
	fn from(array: [T; 2]) -> Self{PairIter{
		offset: 0,
		data: array,
	}}
}
impl<T> Iterator for PairIter<T>{
	type Item = T;

	fn next(&mut self) -> Option<T>{
		if self.offset == 2{
			None
		}else{
			Some(unsafe{ptr::read(self.data.get_unchecked({
				let index = self.offset;
				self.offset+= 1;
				index as usize
			}) as *const T)})
		}
	}
}
