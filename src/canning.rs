
//!
//! Module for canning values.
//! 
//! Canning means wrapping values in some package type, which is better for
//! storing. Thus these Can types contain some `dyn Any` value to allow casting
//! various values into cans.
//! In order to keep them more usable, a Can can be downcasted back to some `T`.
//! 
//! This module also contains a notion for Bins which are 'open' Cans. For
//! instance an `Rc<dyn Any>` as one kind of Can, and its respective Bin is
//! `Rc<T>` for every `T`.
//!  

use std::ops::Deref;
use std::fmt::Debug;
use std::any::Any;



/// Represents an opaque wrapper for `dyn Any`.
/// 
/// This type reperesents a wrapper for a `dyn Any`. It is basis for the `Can`
/// type which allows to be downcasted.
/// 
/// See `Can`.
/// 
pub trait CanBase {
	/// Returns the pointer to the inner value.
	/// 
	fn as_ptr(&self) -> *const dyn Any;
}

/// Represents an opaque wrapper for `dyn Any` which can be casted to `T`.
/// 
/// Since `dyn Any` can't be stored, a `Can` encapsules a `dyn Any` while
/// allowing it to be casted to specific wrapper `Bin` for `T`. It is supposed
/// to be akind `Clone` without requiring it, i.e. any `&Can` needs to be
/// allowed to be casted to `Bin` as value, somehow requireing a clone.
/// 
/// A good example for a `Can` is `Rc<dyn Any>`. Which for any `T` can be casted
/// to a `Rc<T>` which would be the `Bin` type. Additionally, `Rc` allows to be
/// cloned without requiring `T` to be clone.
///
pub trait Can<T: ?Sized>: CanBase {
	/// A specific wrapper for `T` which can be casted from `Self`.
	/// 
	type Bin: Debug;
	
	/// Tries to downcast the opaque `Can` to an specific `Bin`.
	/// 
	/// Because `Can`s are supposed to be alike `Any` allowing various `T`s to
	/// be casted to the same `Can`, this operation inherently may fail.
	/// 
	fn downcast_can(self) -> Option<Self::Bin>;
	
	/// Creates Self form a `Bin`.
	/// 
	/// This is a upcast and can not fail.
	fn from_bin(b: Self::Bin) -> Self;
	
	/// Gets the pointer to 
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any;
}

/// Transparent variant of `Can`.
/// 
/// It allows additional to `Can` to get `T` from `Bin` and directly downcasting
/// this `Can` to `T`.
/// 
pub trait CanTransparent<T>: Can<T> + Sized {
	
	/// Tries to downcast the opaque `Can` to an specific `T`, by passing the
	/// `Bin` and cloning.
	///
	fn downcast_can_ref(&self) -> Option<&T>;

}

/// Mutable transparent variant of `Can`.
/// 
/// It allows additional to `Can` to get `T` from `Bin` and directly downcasting
/// this `Can` to `T`.
/// 
pub trait CanTransparentMut<T>: Can<T> + Sized {
	/// Tries to downcast the opaque `Can` to an specific `T`, by passing the
	/// `Bin` and cloning.
	///
	fn downcast_can_mut(&mut self) -> Option<&mut T>;
	
}

/// Sized variant of `Can`.
///
pub trait CanSized<T>: Can<T> + Sized {
	/// Create a `Bin` for `T`.
	/// 
	fn into_bin(t: T) -> Self::Bin;
	
	/// Create `Self` directly from `T`.
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
	
	fn downcast_can(self) -> Option<Self::Bin> {
		self.downcast().ok()
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

impl<T: Debug + 'static> Can<T> for Box<dyn Any> {
	type Bin = Box<T>;
	
	fn downcast_can(self) -> Option<Self::Bin> {
		self.downcast().ok()
		//	.map(|r: &T| Box::new(r.clone()))
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
	fn bin_as_ptr(b: &Self::Bin) -> *const dyn Any {
		b.deref()
	}
}

impl<T: Debug + 'static> CanTransparent<T> for Box<dyn Any> {
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
}

impl<T: Debug + 'static> CanTransparentMut<T> for Box<dyn Any> {
	fn downcast_can_mut(&mut self) -> Option<&mut T> {
		self.downcast_mut()
	}
}

impl<T: Debug + 'static> CanSized<T> for Box<dyn Any> {
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
	
	fn downcast_can(self) -> Option<Self::Bin> {
		self.downcast().ok()
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
	
	fn downcast_can(self) -> Option<Self::Bin> {
		let id = self.id;
		
		self.builder.downcast_can().map( |bin| {
			Ap {
				builder: bin,
				id: id,
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



