
//!
//! Alias module for using `Box` to wrap artifacts and `Rc` to wrap
//! `Blueprint`.
//!


use std::fmt::Debug;
use std::any::Any;

use cfg_if::cfg_if;

#[cfg(feature = "diagnostics")]
use crate::Doctor;

use crate::BlueprintDyn;
use crate::Never;


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

/// Wraps a Builder as a blueprint for its artifact from the `Cache`.
///
pub type Blueprint<B> = crate::rc::Blueprint<B>;

cfg_if! {
	if #[cfg(feature = "unsized")] {
		/// The unsized variant of `Blueprint`.
		///
		pub type BlueprintUnsized<B> = crate::rc::BlueprintUnsized<B>;
	}
}

/// An `Blueprint` with a `dyn Builder<Artifact=Artifact>`.
///
pub type DynamicBlueprint<Artifact, Err=Never, DynState=()> =
	BlueprintDyn<CanType, BuilderCan, Artifact, Err, DynState>;

pub type ConstBuilder<T> = crate::utils::ConstBuilder<CanType, BuilderCan, BinType<T>, T>;
pub type ConfigurableBuilder<T> = crate::utils::ConfigurableBuilder<CanType, BuilderCan, T>;

/// Allows to resolve any `Blueprint` into its artifact. Usable within a
/// builders `build` function.
///
/// This resolver uses `Rc` for storing builders and and `Box` for artifacts.
///
pub type Resolver<'a, T = ()> = crate::Resolver<'a, CanType, BuilderCan, T>;


cfg_if::cfg_if!{
	if #[cfg(feature = "diagnostics")] {
		/// Allows to resolve any `Blueprint` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and `Box` for artifacts.
		///
		pub type Cache<T = dyn Doctor<CanType, BuilderCan>> =
			crate::Cache<CanType, BuilderCan, T>;
	} else {
		/// Allows to resolve any `Blueprint` into its artifact.
		///
		/// This cache uses `Rc` for storing builders and `Box` for artifacts.
		///
		pub type Cache = crate::Cache<CanType, BuilderCan>;
	}
}

/// The ownable and storable variant of the Cache.
///
pub type CacheOwned = crate::CacheOwned<CanType, BuilderCan>;



/// Functional builder wrapper.
///
pub type FunctionalBuilder<F, S = ()> =
	crate::utils::FunctionalBuilder<CanType, BuilderCan, F, S>;



/// Simplified builder without a dynamic state.
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
			-> Result<BinType<Self::Artifact>, Never> {

		Ok(BinType::new(self.build(cache)))
	}

	fn init_dyn_state(&self) -> Self::DynState {
		// Intensional empty, just return a fresh `()`
	}
}


/// A Builder using `Rc` for `Blueprint` and `Box` for artifacts.
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
		-> Result<BinType<Self::Artifact>, Self::Err>;

	/// Return an inital dynamic state for this builder.
	///
	fn init_dyn_state(&self) -> Self::DynState;
}

impl<B: ?Sized + Builder> crate::Builder<CanType, crate::rc::CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;
	type Err = B::Err;

	fn build(&self, cache: &mut Resolver<Self::DynState>)
			-> Result<<CanType as crate::canning::Can<Self::Artifact>>::Bin, Self::Err> {

		self.build(cache)
	}

	fn init_dyn_state(&self) -> Self::DynState {
		self.init_dyn_state()
	}
}



#[cfg(test)]
mod test_cloned {
	include!("test_impl_cloned.rs");
}




