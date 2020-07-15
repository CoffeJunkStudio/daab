
//!
//! Alias module for using `Rc` to wrap `ArtifactPromise` and artifacts.
//! 

use std::fmt::Debug;
use std::any::Any;

#[cfg(feature = "diagnostics")]
use crate::Doctor;

use crate::BuilderEntry;


/// Type for wrapping a `T` as part of `CanType` as `Can`.
/// 
/// This is just an alias for `Arc<T>`.
///
pub type BinType<T> = std::sync::Arc<T>;

/// Can for wrappers of this module.
/// 
/// This is just an alias for `Arc<dyn Any + Send + Sync>`.
/// 
pub type CanType = BinType<dyn Any + Send + Sync>;


/// Promise for the artifact of the builder `B`, usable at the `ArtifactCache`.
///
/// This promise uses `Arc` for storing the builder, this allows cloning.
///
pub type ArtifactPromise<B> = crate::ArtifactPromise<B, CanType>;


pub type DynamicArtifactPromise<Artifact> =
	crate::ArtifactPromiseUnsized<dyn Builder<Artifact=Artifact, DynState=()> + Send + Sync, CanType>;


/// Allows to resolve any `ArtifactPromis` into its artifact. Usable within a
/// builders `build` function.
/// 
/// This resolver uses `Arc` for storing builders and artifacts.
/// 
pub type ArtifactResolver<'a, T = ()> = crate::ArtifactResolver<'a, CanType, CanType, T>;


/// Allows to resolve any `ArtifactPromis` into its artifact-builder. Usable
/// within a super-builders `build` function.
/// 
/// This resolver uses `Arc` for storing builders and `ArtifactPromise` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
pub type SuperArtifactResolver<'a, T = ()> = crate::ArtifactResolver<'a, BuilderEntry<CanType>, CanType, T>;


cfg_if::cfg_if!{
	if #[cfg(feature = "diagnostics")] {
		/// Allows to resolve any `ArtifactPromis` into its artifact.
		///
		/// This cache uses `Arc` for storing builders and artifacts.
		///
		pub type ArtifactCache<T = dyn Doctor<CanType, CanType>> =
			crate::ArtifactCache<CanType, CanType, T>;

	} else {
		/// Allows to resolve any `ArtifactPromis` into its artifact.
		///
		/// This cache uses `Arc` for storing builders and artifacts.
		///
		pub type ArtifactCache = crate::ArtifactCache<CanType, CanType>;
	}
}



/// Allows to resolve any `ArtifactPromis` into its artifact-builder.
/// 
/// This cache uses `Arc` for storing builders and `ArtifactPromise` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
#[cfg(not(feature = "diagnostics"))]
pub type SuperArtifactCache = crate::ArtifactCache<BuilderEntry<CanType>, CanType>;

/// Allows to resolve any `ArtifactPromis` into its artifact-builder.
/// 
/// This cache uses `Arc` for storing builders and `ArtifactPromise` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
#[cfg(feature = "diagnostics")]
pub type SuperArtifactCache<T = dyn Doctor<BuilderEntry<CanType>, CanType>> = crate::ArtifactCache<BuilderEntry<CanType>, CanType, T>;


/// A simplified builder interface, intended for implementing builders.
///
/// For this trait exists a generic `impl Builder`.
///
pub trait SimpleBuilder: Debug {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + Send + Sync + 'static;
	
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


/// A Builder using `Arc` for `ArtifactPromise` and artifacts.
///
pub trait Builder: Debug {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + Send + Sync + 'static;
	
	/// Type of the dynamic state of this builder.
	/// 
	type DynState : Debug + 'static;

	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut ArtifactResolver<Self::DynState>) -> Self::Artifact;
}

impl<B: ?Sized + Builder> crate::Builder<CanType, CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;

	fn build(&self, cache: &mut ArtifactResolver<Self::DynState>) -> Self::Artifact {
		self.build(cache)
	}
}

/// A builder of builders using `Arc`s.
/// 
/// This cache uses `Arc` for storing builders and `ArtifactPromise` for
/// storing artifacts, i.e. the artifacts are builders them self.
///
pub trait SuperBuilder: Debug + Send + Sync {
	/// The artifact type as produced by this builder. It is supposed to be
	/// another `Builder` (or `SuperBuilder`).
	///
	type Artifact : Debug + Send + Sync + 'static;
	
	/// Type of the dynamic state of this builder.
	/// 
	type DynState : Debug + 'static;

	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut SuperArtifactResolver<Self::DynState>) -> Self::Artifact;
}

impl<B: ?Sized + SuperBuilder> crate::Builder<crate::BuilderEntry<CanType>, CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;

	fn build(&self, cache: &mut SuperArtifactResolver<Self::DynState>) -> Self::Artifact {
		self.build(cache)
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



