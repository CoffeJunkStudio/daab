
//!
//! Alias module for using `Box` to wrap artifacts and `Rc` to wrap
//! `ArtifactPromise`.
//!


use std::fmt::Debug;
use std::any::Any;
use std::fmt;

#[cfg(feature = "diagnostics")]
use crate::Doctor;

pub use crate::rc::CanType as RcCanType;


/// Type for wrapping a `T` as part of `CanType` as `Can`.
///
/// This is just an alias for `Box<T>`.
///
pub type BinType<T> = Box<T>;

/// Can for wrappers of this module.
///
/// This is just an alias for `Box<dyn Any>`.
///
pub type CanType = BinType<dyn Any>;

pub type BuilderBinType<T> = crate::rc::BinType<T>;


/// Promise for the artifact of the builder `B`, usable at the `ArtifactCache`.
///
/// This promise uses `Rc` for storing the builder, this allows cloning.
///
pub type ArtifactPromise<B> = crate::rc::ArtifactPromise<B>;

pub type ArtifactPromiseUnsized<B> = crate::rc::ArtifactPromiseUnsized<B>;

pub type DynamicArtifactPromise<Artifact> =
	ArtifactPromiseUnsized<dyn Builder<Artifact=Artifact, DynState=()>>;


/// Allows to resolve any `ArtifactPromis` into its artifact. Usable within a
/// builders `build` function.
///
/// This resolver uses `Rc` for storing builders and and `Box` for artifacts.
///
pub type ArtifactResolver<'a, T = ()> = crate::ArtifactResolver<'a, CanType, RcCanType, T>;


cfg_if::cfg_if!{
	if #[cfg(feature = "diagnostics")] {
		/// Allows to resolve any `ArtifactPromis` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and `Box` for artifacts.
		///
		pub type ArtifactCache<T = dyn Doctor<CanType, RcCanType>> =
			crate::ArtifactCache<CanType, RcCanType, T>;
	} else {
		/// Allows to resolve any `ArtifactPromis` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and `Box` for artifacts.
		///
		pub type ArtifactCache = crate::ArtifactCache<CanType, RcCanType>;
	}
}

// TODO: dyn state rewrite

/// Functional builder wrapper.
///
/// A functional builder is a builder consisting of a single function
/// `Fn(&mut ArtifactResolver) -> T`. Thus this type can be used to wrap a
/// closure as `Builder`. The return type `T` will the artifact type of the
/// resulting Builder.
///
pub struct FunctionalBuilder<F> {
	inner: F,
}

impl<F> Debug for FunctionalBuilder<F> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "FunctionalBuilder{{...}}")
	}
}

impl<F,T> FunctionalBuilder<F>
	where F: Fn(&mut ArtifactResolver) -> T,
		T: Debug + 'static {

	/// Wraps the given closure as Builder.
	///
	pub fn new(f: F) -> Self {
		FunctionalBuilder {
			inner: f,
		}
	}
}

impl<F: 'static, T: Debug + 'static> From<F> for ArtifactPromise<FunctionalBuilder<F>>
	where F: for<'r, 's> Fn(&'r mut ArtifactResolver<'s>) -> T, {

	fn from(f: F) -> Self {
		ArtifactPromise::new(
			FunctionalBuilder::new(f)
		)
	}
}

impl<F,T> Builder for FunctionalBuilder<F>
	where F: Fn(&mut ArtifactResolver) -> T,
		T: Debug + 'static {

	type Artifact = T;
	type DynState = ();

	fn build(&self, resolver: &mut ArtifactResolver)
			 -> Self::Artifact {

		let f = &self.inner;
		f(resolver)

	}
}


/// Simplified builder without a dynamic state.
pub trait SimpleBuilder: Debug {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;

	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut ArtifactResolver) -> Self::Artifact;
}

// Generic impl for legacy builder
impl<B: ?Sized + SimpleBuilder> Builder for B {
	type Artifact = B::Artifact;

	type DynState = ();

	fn build(&self, cache: &mut ArtifactResolver) -> Self::Artifact {
		self.build(cache)
	}
}


/// A Builder using `Rc` for `ArtifactPromise` and `Box` for artifacts.
///
pub trait Builder: Debug {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;

	/// Type of the dynamic state of this builder.
	///
	type DynState : Debug + 'static;

	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut ArtifactResolver<Self::DynState>) -> Self::Artifact;
}

impl<B: ?Sized + Builder> crate::Builder<CanType, crate::rc::CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;

	fn build(&self, cache: &mut ArtifactResolver<Self::DynState>) -> Self::Artifact {
		self.build(cache)
	}
}



#[cfg(test)]
mod test_cloned {
	include!("test_impl_cloned.rs");
}




