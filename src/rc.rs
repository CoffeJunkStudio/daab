
//!
//! Alias module for using `Rc` to wrap `Blueprint` and artifacts.
//!

use std::fmt::Debug;
use std::any::Any;

#[cfg(feature = "diagnostics")]
use crate::Doctor;

use crate::canning::BuilderArtifact;
use crate::Never;


/// Type for wrapping a `T` as part of `CanType` as `Can`.
///
/// This is just an alias for `Rc<T>`.
///
pub type BinType<T> = std::rc::Rc<T>;

/// Can for wrappers of this module.
///
/// This is just an alias for `Rc<dyn Any>`.
///
pub type CanType = BinType<dyn Any>;

/// The wrapping type for builders.
///
/// Here it `Rc<dyn Any> as opposed to `BinType`.
///
pub type BuilderBinType<T> = BinType<T>;

/// The can type for builders.
///
pub type BuilderCan = CanType;

/// Wraps a Builder as a blueprint for its artifact from the `Cache`.
///
pub type Blueprint<B> = crate::Blueprint<B, CanType>;

/// The unsized variant of `Blueprint`.
///
pub type BlueprintUnsized<B> = crate::BlueprintUnsized<B, CanType>;

/// An `Blueprint` with a `dyn Builder<Artifact=Artifact>`.
///
pub type DynamicBlueprint<Artifact> =
	BlueprintUnsized<dyn Builder<Artifact=Artifact, DynState=(), Err=Never>>;


/// Allows to resolve any `ArtifactPromis` into its artifact. Usable within a
/// builders `build` function.
///
/// This resolver uses `Rc` for storing builders and artifacts.
///
pub type Resolver<'a, T = ()> = crate::Resolver<'a, CanType, CanType, T>;


/// Allows to resolve any `ArtifactPromis` into its artifact-builder. Usable
/// within a super-builders `build` function.
///
/// This resolver uses `Rc` for storing builders and `Blueprint` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
pub type SuperResolver<'a, T = ()> = crate::Resolver<'a, BuilderArtifact<CanType>, CanType, T>;


cfg_if::cfg_if!{
	if #[cfg(feature = "diagnostics")] {
		/// Allows to resolve any `ArtifactPromis` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and artifacts.
		///
		pub type Cache<T = dyn Doctor<CanType, CanType>> =
			crate::Cache<CanType, CanType, T>;

	} else {
		/// Allows to resolve any `ArtifactPromis` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and artifacts.
		///
		pub type Cache = crate::Cache<CanType, CanType>;
	}
}

/// The ownable and storable variant of the Cache.
///
pub type CacheOwned = crate::CacheOwned<CanType, CanType>;



/// Allows to resolve any `ArtifactPromis` into its artifact-builder.
///
/// This cache uses `Rc` for storing builders and `Blueprint` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
#[cfg(not(feature = "diagnostics"))]
pub type SuperCache = crate::Cache<BuilderArtifact<CanType>, CanType>;

/// Allows to resolve any `ArtifactPromis` into its artifact-builder.
///
/// This cache uses `Rc` for storing builders and `Blueprint` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
#[cfg(feature = "diagnostics")]
pub type SuperCache<T = dyn Doctor<BuilderArtifact<CanType>, CanType>> = crate::Cache<BuilderArtifact<CanType>, CanType, T>;


/// Functional builder wrapper.
///
pub type FunctionalBuilder<F, S = ()> =
	crate::utils::FunctionalBuilder<CanType, BuilderCan, F, S>;


/// A simplified builder interface, intended for implementing builders.
///
/// For this trait exists a generic `impl Builder`.
///
pub trait SimpleBuilder: Debug + 'static {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;

	/// Produces an artifact using the given `Resolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut Resolver) -> Self::Artifact;
}

// Generic impl for legacy builder
impl<B: ?Sized + SimpleBuilder> Builder for B {
	type Artifact = B::Artifact;

	type DynState = ();

	type Err = Never;

	fn build(&self, cache: &mut Resolver)
			-> Result<Self::Artifact, Never> {

		Ok(self.build(cache))
	}

	fn init_dyn_state(&self) -> Self::DynState {
		// Intensional empty, just return a fresh `()`
	}
}


/// A Builder using `Rc` for `Blueprint` and artifacts.
///
pub trait Builder: Debug + 'static {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;

	/// Type of the dynamic state of this builder.
	///
	type DynState : Debug + 'static;

	/// Error type returned by this Builder in case of failure to produce an
	/// Artifact.
	type Err : Debug + 'static;

	/// Produces an artifact using the given `Resolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut Resolver<Self::DynState>)
		-> Result<Self::Artifact, Self::Err>;

	/// Return an inital dynamic state for this builder.
	///
	fn init_dyn_state(&self) -> Self::DynState;
}

impl<B: ?Sized + Builder> crate::Builder<CanType, CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;
	type Err = B::Err;

	fn build(&self, cache: &mut Resolver<Self::DynState>)
			-> Result<Self::Artifact, Self::Err> {

		self.build(cache)
	}

	fn init_dyn_state(&self) -> Self::DynState {
		self.init_dyn_state()
	}
}

/// A builder of builders using `Rc`s.
///
/// This cache uses `Rc` for storing builders and `Blueprint` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
pub trait SuperBuilder: Debug + 'static {
	/// The artifact type as produced by this builder. It is supposed to be
	/// another `Builder` (or `SuperBuilder`).
	///
	type Artifact : Debug + 'static;

	/// Type of the dynamic state of this builder.
	///
	type DynState : Debug + 'static;

	/// Error type returned by this Builder in case of failure to produce an
	/// Artifact.
	type Err : Debug + 'static;

	/// Produces an artifact using the given `Resolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut SuperResolver<Self::DynState>)
		-> Result<Self::Artifact, Self::Err>;

	/// Return an inital dynamic state for this builder.
	///
	fn init_dyn_state(&self) -> Self::DynState;
}

impl<B: ?Sized + SuperBuilder> crate::Builder<BuilderArtifact<CanType>, CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;
	type Err = B::Err;

	fn build(&self, cache: &mut SuperResolver<Self::DynState>)
			-> Result<Self::Artifact, Self::Err> {

		self.build(cache)
	}

	fn init_dyn_state(&self) -> Self::DynState {
		self.init_dyn_state()
	}
}


#[cfg(test)]
mod test {
	include!("test_impl.rs");
}

#[cfg(test)]
mod test_cloned {
	include!("test_impl_cloned.rs");
}



