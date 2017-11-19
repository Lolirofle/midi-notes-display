pub struct FilteredScanIter<Iter,State,Func>{
	pub iter : Iter,
	pub state: State,
	pub func : Func,
}

impl<Iter,State,T,Func> Iterator for FilteredScanIter<Iter,State,Func> where
	Iter: Iterator,
	Func: FnMut(&mut State, Iter::Item) -> Option<T>,
{
	type Item = T;

	#[inline]
	fn next(&mut self) -> Option<T>{
		while let Some(elem) = self.iter.next(){
			match (self.func)(&mut self.state,elem){
				Some(x) => return Some(x),
				None    => (),
			}
		}
		None
	}

	#[inline(always)]
	fn size_hint(&self) -> (usize,Option<usize>){
		(0,self.iter.size_hint().1)
	}
}

pub trait FilteredScanIteratorExt{
	fn filtered_scan<State,T,Func>(self,initial_state: State,func: Func) -> FilteredScanIter<Self,State,Func> where
		Self: Sized + Iterator,
		Func: FnMut(&mut State,Self::Item) -> Option<T>,
	{
		FilteredScanIter{iter: self , state: initial_state , func: func}
	}
}
impl<Iter: ?Sized> FilteredScanIteratorExt for Iter where Iter: Iterator{}
