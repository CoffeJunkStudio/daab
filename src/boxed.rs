
//!
//! Alias module for using `Rc` to wrap `ArtifactPromise` and artifacts.
//! 

use std::fmt::Debug;
use std::any::Any;

#[cfg(feature = "diagnostics")]
use crate::Doctor;

pub use crate::rc::CanType as RcCanType;

/// Type for wrapping a `T` as part of `CanType` as `Can`.
/// 
/// This is just an alias for `Rc<T>`.
///
pub type BinType<T> = Box<T>;

/// Can for wrappers of this module.
/// 
/// This is just an alias for `Rc<dyn Any>`.
/// 
pub type CanType = BinType<dyn Any>;


/// Promise for the artifact of the builder `B`, usable at the `ArtifactCache`.
///
/// This promise uses `Rc` for storing the builder, this allows cloning.
///
pub type ArtifactPromise<B> = crate::rc::ArtifactPromise<B>;

/// Allows to resolve any `ArtifactPromis` into its artifact. Usable within a
/// builders `build` function.
/// 
/// This resolver uses `Rc` for storing builders and artifacts.
/// 
pub type ArtifactResolver<'a, T = ()> = crate::ArtifactResolver<'a, CanType, RcCanType, T>;

/// Allows to resolve any `ArtifactPromis` into its artifact. 
/// 
/// This cache uses `Rc` for storing builders and artifacts.
/// 
#[cfg(not(feature = "diagnostics"))]
pub type ArtifactCache = crate::ArtifactCache<CanType, RcCanType>;

/// Allows to resolve any `ArtifactPromis` into its artifact. 
/// 
/// This cache uses `Rc` for storing builders and artifacts.
/// 
#[cfg(feature = "diagnostics")]
pub type ArtifactCache<T = dyn Doctor<CanType, CanType>> = crate::ArtifactCache<CanType, RcCanType, T>;


/// A simplified builder interface, intended for implementing builders.
///
/// For this trait exists a generic `impl Builder`.
///
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
impl<B: SimpleBuilder> Builder for B {
	type Artifact = B::Artifact;
	
	type DynState = ();
	
	fn build(&self, cache: &mut ArtifactResolver) -> BinType<Self::Artifact> {
		BinType::new(self.build(cache))
	}
}


/// A Builder using `Rc` for `ArtifactPromise` and artifacts.
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
	fn build(&self, resolver: &mut ArtifactResolver<Self::DynState>) -> BinType<Self::Artifact>;
}

impl<B: Builder> crate::Builder<CanType, crate::rc::CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;
	
	fn build(&self, cache: &mut ArtifactResolver<Self::DynState>) -> BinType<Self::Artifact> {
		self.build(cache)
	}
}


/*
#[cfg(test)]
mod test {
	include!("test_impl.rs");
}
*/


