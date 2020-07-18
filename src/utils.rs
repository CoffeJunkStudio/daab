//!
//! Utility module
//!
//! This module contains some utilities which can be useful when working with `daab`.
//!

use crate::Resolver;
use crate::Promise;
use crate::Blueprint;
use crate::Builder;
use crate::CanRef;
use crate::CanStrong;
use crate::CanSized;

use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;



/// A intermediate cached Builder to circumvent failing builders.
///
/// In resource loading, a resource might be come unavailable for sometime.
/// This wrapper builder will cache the result of its inner builder and return
/// the cached value instead, if the inner builder failed to produce a new
/// artifact.
///
/// **Notice: It is likely a logical error to keep an artifact despite
/// been invalidate, which is what this wrapper dose!**
///
/// Therefore, this wrapper should be used with care and only
/// with builders, which actually do not have any dependencies (so they might
/// only been manually invalidate) or having artifacts which are still usable
/// after any (or all) dependency has been invalidated.
///
#[derive(Debug, Clone)]
pub struct FallibleBuilder<AP, B: ?Sized, T> {
	inner: AP,
	default_value: Option<T>,
	_b: PhantomData<B>,
}

impl<AP, B: ?Sized, T> FallibleBuilder<AP, B, T>
	where
		T: Clone,
		B: Debug + 'static, {

	/// Wrap given Builder and fill missing recreations with a previous value.
	///
	/// **Use with care**
	///
	pub fn new<ArtCan, BCan>(
		inner: AP,
		default_value: Option<T>
	) -> Blueprint<Self, BCan>
		where
			B: Builder<ArtCan, BCan, Artifact=Option<T>>,
			AP: Promise<B, BCan>,
			T: Clone + Debug + 'static,
			ArtCan: CanSized<Option<T>> + CanRef<Option<T>>,
			ArtCan::Bin: AsRef<Option<T>>,
			BCan: Clone + CanStrong,
			BCan: CanSized<Self>,
	{

		Blueprint::new(
			FallibleBuilder {
				inner,
				default_value,
				_b: PhantomData,
			}
		)
	}
}

impl<ArtCan, AP, B: ?Sized, BCan, T> Builder<ArtCan, BCan> for FallibleBuilder<AP, B, T>
	where
		B: Builder<ArtCan, BCan, Artifact=Option<T>>,
		AP: Promise<B, BCan>,
		T: Clone + Debug + 'static,
		ArtCan: CanSized<Option<T>> + CanRef<Option<T>>,
		ArtCan::Bin: AsRef<Option<T>>,
		BCan: Clone + CanStrong,
	{

	type Artifact = T;
	type DynState = Option<T>;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>) -> Self::Artifact {

		let value = resolver.resolve_cloned(&self.inner);

		if let Some(v) = value {
			*resolver.my_state() = Some(v.clone());

			// Return value
			v

		} else {
			// Try to return cached value. Panics if very first build fails.
			resolver.my_state().clone().unwrap()
		}
	}
	
	fn init_dyn_state(&self) -> Self::DynState {
		self.default_value.clone()
	}
}




/// Functional builder wrapper.
///
/// A functional builder is a builder consisting of a single function
/// `Fn(&mut Resolver) -> T`. Thus this type can be used to wrap a
/// closure as `Builder`. The return type `T` will the artifact type of the
/// resulting Builder.
///
pub struct FunctionalBuilder<ArtCan, BCan, F> {
	inner: F,
	_art_can: PhantomData<ArtCan>,
	_b_can: PhantomData<BCan>,
}

impl<ArtCan, BCan, F> Debug for FunctionalBuilder<ArtCan, BCan, F> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "FunctionalBuilder{{...}}")
	}
}

impl<ArtCan, BCan, F, T> FunctionalBuilder<ArtCan, BCan, F>
	where
		F: (for<'r, 's> Fn(&'r mut Resolver<'s, ArtCan, BCan>) -> T) + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		ArtCan: 'static {

	/// Wraps the given closure as Builder.
	///
	pub fn new(f: F) -> Self {
		FunctionalBuilder {
			inner: f,
			_art_can: PhantomData,
			_b_can: PhantomData,
		}
	}
}

impl<ArtCan, BCan, F: 'static, T: Debug + 'static> From<F> for Blueprint<FunctionalBuilder<ArtCan, BCan, F>, BCan>
	where
		F: (for<'r, 's> Fn(&'r mut Resolver<'s, ArtCan, BCan>) -> T) + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		BCan: CanSized<FunctionalBuilder<ArtCan, BCan, F>>,
		ArtCan: 'static {

	fn from(f: F) -> Self {
		Blueprint::new(
			FunctionalBuilder::new(f)
		)
	}
}

impl<ArtCan, BCan, F, T> Builder<ArtCan, BCan> for FunctionalBuilder<ArtCan, BCan, F>
	where
		F: (for<'r, 's> Fn(&'r mut Resolver<'s, ArtCan, BCan>) -> T) + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		ArtCan: 'static {

	type Artifact = T;
	type DynState = ();

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan>)
			 -> Self::Artifact {

		let f = &self.inner;
		f(resolver)

	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}

