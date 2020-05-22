
//!
//! Adds additional wrappers for `ArtifactCache` and `ArtifactResolver`,
//! which allows to get directly a clone of the value.
//!

use std::ops::Deref;
use std::ops::DerefMut;
use std::fmt::Debug;

use super::ArtifactResolver;
use super::ArtifactCache;
use super::ArtifactPromise;
use super::Builder;
use super::CanType;
use super::RcCanType;


/// Box Artifact recolver which extracts the box.
pub struct ArtifactResolverWrapped<'a, 'b, T = ()>(
	pub &'a mut ArtifactResolver<'b, T>
);

#[cfg(feature = "diagnostics")]
use crate::diagnostics::Doctor;

#[cfg(feature = "diagnostics")]
pub struct ArtifactCacheWrapped<T: ?Sized = dyn Doctor<CanType, RcCanType>>(
	pub ArtifactCache<T>
);
#[cfg(not(feature = "diagnostics"))]
pub struct ArtifactCacheWrapped(
	pub ArtifactCache
);


#[cfg(feature = "diagnostics")]
impl<T: Doctor<CanType, RcCanType> + 'static> ArtifactCacheWrapped<T> {
	pub fn new_with_doc(doc: T) -> Self {
		Self(super::ArtifactCache::new_with_doctor(doc).into())
	}
}

#[cfg(feature = "diagnostics")]
impl ArtifactCacheWrapped<crate::DefDoctor> {
	/// Creates a new empty cache with a dummy doctor.
	///
	pub fn new() -> Self {
		Self(super::ArtifactCache::new())
	}
}

#[cfg(not(feature = "diagnostics"))]
impl ArtifactCacheWrapped {
	pub fn new() -> Self {
		Self(super::ArtifactCache::new())
	}
}

impl Deref for ArtifactCacheWrapped {
	type Target = ArtifactCache;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl DerefMut for ArtifactCacheWrapped {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl ArtifactCacheWrapped {
	/// Get and cast the stored artifact if it exists.
	///
	/// Also see `ArtifactCache::lookup_cloned`.
	///
	pub fn lookup<B: Builder + 'static>(
			&self,
			builder: &ArtifactPromise<B>
		) -> Option<B::Artifact>
			where B::Artifact: Clone
			{

		self.0.lookup_ref(builder).cloned()

	}

	pub fn get<B: Builder + 'static>(
			&mut self,
			promise: &ArtifactPromise<B>
		) -> B::Artifact where B::Artifact: Clone {


		self.0.get_ref(promise).clone()
	}
}




impl<'a, 'b> Deref for ArtifactResolverWrapped<'a, 'b> {
	type Target = ArtifactResolver<'a>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}
impl<'b: 'a, 'a: 'b> DerefMut for ArtifactResolverWrapped<'a, 'b> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.0
	}
}

impl<'a, 'b, T: 'static> ArtifactResolverWrapped<'a, 'b, T> {

	/// Resolves the given `ArtifactPromise` into a clone of its artifact by
	/// using `resolve_ref()` and `clone().
	///
	/// Also see `ArtifactResolver::resolve`
	///
	pub fn resolve<B: Builder + 'static>(
			&mut self,
			promise: &ArtifactPromise<B>
		) -> B::Artifact where B::Artifact: Clone {

		self.0.resolve_ref(promise).clone()
	}
}


/// A wrapped builder interface, intended for implementing builders.
///
/// For this trait exists a generic `impl Builder`.
///
pub trait BuilderWrapped: Debug {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;

	/// Type of the dynamic state of this builder.
	///
	type DynState: Debug + 'static;

	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut ArtifactResolverWrapped<Self::DynState>) -> Self::Artifact;
}

// Generic impl for legacy builder
impl<B: BuilderWrapped> super::Builder for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;

	fn build(&self, cache: &mut super::ArtifactResolver<Self::DynState>) -> Self::Artifact {
		{
			let mut c = ArtifactResolverWrapped(cache);

			return self.build(&mut c)
		}
	}
}
