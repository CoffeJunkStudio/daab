
//!
//! Alias module for using `Box` to wrap artifacts and `Rc` to wrap
//! `ArtifactPromise`.
//!


use std::fmt::Debug;
use std::any::Any;

#[cfg(feature = "diagnostics")]
use crate::Doctor;


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

/// The wrapping type for builders.
///
/// Here it `Rc<dyn Any> as opposed to `BinType`.
///
pub type BuilderBinType<T> = crate::rc::BinType<T>;

/// The can type for builders.
///
pub type BuilderCan = crate::rc::CanType;


/// Promise for the artifact of the builder `B`, usable at the `ArtifactCache`.
///
/// This promise uses `Rc` for storing the builder, this allows cloning.
///
pub type ArtifactPromise<B> = crate::rc::ArtifactPromise<B>;

/// The unsized variant of `ArtifactPromise`.
///
pub type ArtifactPromiseUnsized<B> = crate::rc::ArtifactPromiseUnsized<B>;

/// An `ArtifactPromise` with a `dyn Builder<Artifact=Artifact>`.
///
pub type DynamicArtifactPromise<Artifact> =
	ArtifactPromiseUnsized<dyn crate::Builder<CanType, BuilderBinType<dyn Any>,Artifact=Artifact, DynState=()>>;


/// Allows to resolve any `ArtifactPromise` into its artifact. Usable within a
/// builders `build` function.
///
/// This resolver uses `Rc` for storing builders and and `Box` for artifacts.
///
pub type ArtifactResolver<'a, T = ()> = crate::ArtifactResolver<'a, CanType, BuilderCan, T>;


cfg_if::cfg_if!{
	if #[cfg(feature = "diagnostics")] {
		/// Allows to resolve any `ArtifactPromise` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and `Box` for artifacts.
		///
		pub type ArtifactCache<T = dyn Doctor<CanType, BuilderCan>> =
			crate::ArtifactCache<CanType, BuilderCan, T>;
	} else {
		/// Allows to resolve any `ArtifactPromise` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and `Box` for artifacts.
		///
		pub type ArtifactCache = crate::ArtifactCache<CanType, BuilderCan>;
	}
}


/// Functional builder wrapper.
///
pub type FunctionalBuilder<F> =
	crate::utils::FunctionalBuilder<CanType, BuilderCan, F>;



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




