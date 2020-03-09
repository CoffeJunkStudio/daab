


use std::ops::Deref;
use std::fmt::Debug;
use std::any::Any;



pub trait CanBase {
	fn as_ptr(&self) -> *const dyn Any;
}

/// A can for `T`. Supposed to be akind to `dyn Any`.
// For Artifacts
pub trait Can<T: ?Sized>: CanBase {
	type Bin: Debug;
	
	fn downcast_can(&self) -> Option<Self::Bin>;
	fn from_bin(b: Self::Bin) -> Self;
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any;
}

pub trait CanTransparent<T>: Can<T> + Sized where Self::Bin: AsRef<T> {
	
	fn downcast_can_ref(&self) -> Option<&T>;
	
}
pub trait CanSized<T>: Can<T> + Sized {
	fn into_bin(t: T) -> Self::Bin;
	fn from_inner(t: T) -> Self {
		Self::from_bin(Self::into_bin(t))
	}
}


use std::rc::Rc;

impl CanBase for Rc<dyn Any> {
	fn as_ptr(&self) -> *const dyn Any {
		self
	}
}

impl<T: Debug + 'static> Can<T> for Rc<dyn Any> {
	type Bin = Rc<T>;
	
	fn downcast_can(&self) -> Option<Self::Bin> {
		self.clone().downcast().ok()
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b.deref()
	}
}

impl<T: Debug + 'static> CanTransparent<T> for Rc<dyn Any> {
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
}

impl<T: Debug + 'static> CanSized<T> for Rc<dyn Any> {
	fn into_bin(t: T) -> Self::Bin {
		Rc::new(t)
	}
}


impl CanBase for Box<dyn Any> {
	fn as_ptr(&self) -> *const dyn Any {
		self
	}
}

impl<T: Debug + Clone + 'static> Can<T> for Box<dyn Any> {
	type Bin = Box<T>;
	
	fn downcast_can(&self) -> Option<Self::Bin> {
		self.downcast_ref()
			.map(|r: &T| Box::new(r.clone()))
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b.deref()
	}
}

impl<T: Debug + Clone + 'static> CanTransparent<T> for Box<dyn Any> {
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
}

impl<T: Debug + Clone + 'static> CanSized<T> for Box<dyn Any> {
	fn into_bin(t: T) -> Self::Bin {
		Box::new(t)
	}
}


// TODO: impl for AP, Arc, maybe T/Box

use std::sync::Arc;

impl CanBase for Arc<dyn Any + Send + Sync> {
	fn as_ptr(&self) -> *const dyn Any {
		self
	}
}

impl<T: Debug + Send + Sync + 'static> Can<T> for Arc<dyn Any + Send + Sync> {
	type Bin = Arc<T>;
	
	fn downcast_can(&self) -> Option<Self::Bin> {
		self.clone().downcast().ok()
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b.deref()
	}
}

impl<T: Debug + Send + Sync + 'static> CanTransparent<T> for Arc<dyn Any + Send + Sync> {
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
}

impl<T: Debug + Send + Sync + 'static> CanSized<T> for Arc<dyn Any + Send + Sync> {
	fn into_bin(t: T) -> Self::Bin {
		Arc::new(t)
	}
}



use crate::ArtifactPromise as Ap;
use crate::BuilderEntry;

impl<BCan: CanBase + 'static> CanBase for BuilderEntry<BCan> {
	fn as_ptr(&self) -> *const dyn Any {
		self
	}
}

impl<BCan: 'static, B: 'static> Can<B> for BuilderEntry<BCan>
		where BCan: Can<B> {
	
	type Bin = Ap<B, BCan>;
	
	fn downcast_can(&self) -> Option<Self::Bin> {
		self.builder.downcast_can().map( |bin| {
			Ap {
				builder: bin,
				id: self.id,
			}
		})
	}
	fn from_bin(b: Self::Bin) -> Self {
		BuilderEntry::new(b)
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b
	}
}

impl<BCan: 'static, B: 'static> CanSized<B> for BuilderEntry<BCan>
		where BCan: CanSized<B> {
	fn into_bin(t: B) -> Self::Bin {
		Ap::new(t)
	}
}



