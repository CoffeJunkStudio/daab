


use std::ops::Deref;
use std::fmt::Debug;
use std::any::Any;



pub trait CanBase: Debug {
	fn as_ptr(&self) -> *const dyn Any;
}

/// A can for `T`. Supposed to be akind to `dyn Any`.
// For Artifacts
pub trait Can<T: ?Sized>: CanBase {
	type Bin: Debug + AsRef<T>;
	
	fn downcast_can(&self) -> Option<Self::Bin>;
	fn downcast_can_ref(&self) -> Option<&T>;
	fn from_bin(b: Self::Bin) -> Self;
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any;
}

pub trait CanDeref<T>: Can<T> where Self::Bin: Deref<Target=T> {
	
}

pub trait CanWithSize<T>: Can<T> + Sized {
	fn into_bin(t: T) -> Self::Bin;
	fn from_inner(t: T) -> Self {
		Self::from_bin(Self::into_bin(t))
	}
}


use std::rc::Rc;

impl<T: Debug + 'static> Can<T> for Rc<dyn Any> {
	type Bin = Rc<T>;
	
	fn downcast_can(&self) -> Option<Self::Bin> {
		self.clone().downcast().ok()
	}
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b.deref()
	}
}

impl<T: Debug + 'static> CanWithSize<T> for Rc<dyn Any> {
	fn into_bin(t: T) -> Self::Bin {
		Rc::new(t)
	}
}

impl CanBase for Rc<dyn Any> {
	fn as_ptr(&self) -> *const dyn Any {
		self
	}
}


// TODO: impl for AP, Arc, maybe T/Box

use std::sync::Arc;

impl CanBase for Arc<dyn Any + Send + Sync> {
	fn as_ptr(&self) -> *const dyn std::any::Any {
		self
	}
}

impl<T: Debug + Send + Sync + 'static> Can<T> for Arc<dyn Any + Send + Sync> {
	type Bin = Arc<T>;
	
	fn downcast_can(&self) -> Option<Self::Bin> {
		self.clone().downcast().ok()
	}
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b.deref()
	}
}

impl<T: Debug + Send + Sync + 'static> CanWithSize<T> for Arc<dyn Any + Send + Sync> {
	fn into_bin(t: T) -> Self::Bin {
		Arc::new(t)
	}
}


/*
use crate::ArtifactPromise as Ap;

impl<BCan: CanBase + 'static> CanBase for BuilderEntry<BCan> {
	fn as_ptr(&self) -> *const dyn std::any::Any {
		self
	}
}

impl<BCan: CanBase + 'static, T: 'static> Can<T> for BuilderEntry<BCan> where BCan: Can<T> {
	type Bin = Ap<BCan::Bin>;
	
	fn downcast_can_ref(&self) -> Option<&Self::Bin> {
		self.downcast_ref()
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b
	}
}

impl<T: Send + Sync + 'static> CanWithSize<T> for Ap<dyn Any + Send + Sync> {
	fn into_bin(t: T) -> Self::Bin {
		Arc::new(t)
	}
}
*/


