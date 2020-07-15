

#![cfg_attr(feature = "unsized", feature(unsize))]

//!
//! DAG Aware Artifact Builder
//! ==========================
//!
//! Rust crate for managing the building of artifacts by builders which are
//! connected in a directed acyclic graph (DAG) like manner.
//!
//! This crate provides essentially a cache which keeps artifacts of builders in
//! order to prevent the same builder to produce multiple equal artifacts.
//! This could be useful if the builders use consumable resources to create their
//! artifacts, the building is a heavyweight procedure, or a given DAG dependency
//! structure among the builders shall be properly preserved among their
//! artifacts.
//!
//! The basic principal on which this crate is build, suggests two levels of
//! abstraction, the builder level and the artifact level. Each builder type has
//! one specific artifact type. The builders are represented by any struct,
//! which implements the [`Builder`] trait, which in turn has an associate type
//! that specifies the artifact type.
//!
//! `Builder`s are supposed to be wrapped in [`ArtifactPromise`]s, which prevents
//! to call its `Builder::build()` method directly. In other respects, the
//! `ArtifactPromise` acts a lot like an `Rc` and thus allows to share one
//! instance among several dependants.
//! This `Rc`-like structure creates naturally a DAG.
//!
//! For building a `Builder`s artifact, its `Builder::build()` method is
//! provided with a [`ArtifactResolver`] that allows to resolve depending
//! `ArtifactPromise`s into their respective artifacts, which is,
//! in order to form a DAG, wrapped behind a `Rc`.
//!
//! As entry point serves the [`ArtifactCache`], which allows outside of a
//! `Builder` to resolve any `ArtifactPromise` to its artifact. The
//! `ArtifactCache` is essentially a cache for artifacts. It can be used to
//! translate any number of `ArtifactPromise`s to their respective artifact,
//! while sharing their common dependencies.
//! Consequently, resolving the same `ArtifactPromise` using the same
//! `ArtifactCache` results in the same `Rc`ed artifact.
//! However, using different `ArtifactCache`s results in different artifacts.
//!
//! The `ArtifactCache` has a `clear()` method to reset the cache.
//! This could be useful to free the resources kept by all artifacts and
//! builders, which are cached in it, or when artifacts shall be explicitly
//! recreated, e.g. to form a second independent artifact DAG.
//! Additionally, `ArtifactCache` has an `invalidate()` method to remove a single
//! builder and artifact including its dependants (i.e. those artifacts which had
//! used the invalidated one).
//!
//![`Builder`]: trait.Builder.html
//![`ArtifactPromise`]: struct.ArtifactPromise.html
//![`ArtifactResolver`]: struct.ArtifactResolver.html
//![`ArtifactCache`]: struct.ArtifactCache.html
//!
//! Minimal Rust version: **1.40**
//!
//!
//!
//! ## Example
//!
//! ```rust
//! use std::rc::Rc;
//! use daab::*;
//!
//! // Simple artifact
//! #[derive(Debug)]
//! struct Leaf {
//!     //...
//! }
//!
//! // Simple builder
//! #[derive(Debug)]
//! struct BuilderLeaf {
//!     // ...
//! }
//! impl BuilderLeaf {
//!     pub fn new() -> Self {
//!         Self {
//!             // ...
//!         }
//!     }
//! }
//! impl rc::SimpleBuilder for BuilderLeaf {
//!     type Artifact = Leaf;
//!
//!     fn build(&self, _resolver: &mut rc::ArtifactResolver) -> Self::Artifact {
//!         Leaf{
//!             // ...
//!         }
//!     }
//! }
//!
//! // Composed artifact, linking to a Leaf
//! #[derive(Debug)]
//! struct Node {
//!     leaf: Rc<Leaf>, // Dependency artifact
//!     value: u8, // Some custom value
//!     // ...
//! }
//!
//! // Composed builder, depending on BuilderLeaf
//! #[derive(Debug)]
//! struct BuilderNode {
//!     builder_leaf: rc::ArtifactPromise<BuilderLeaf>, // Dependency builder
//!     // ...
//! }
//! impl BuilderNode {
//!     pub fn new(builder_leaf: rc::ArtifactPromise<BuilderLeaf>) -> Self {
//!         Self {
//!             builder_leaf,
//!             // ...
//!         }
//!     }
//! }
//! use std::any::Any;
//! impl rc::Builder for BuilderNode {
//!     type Artifact = Node;
//!     type DynState = u8;
//!
//!     fn build(&self, resolver: &mut rc::ArtifactResolver<Self::DynState>) -> Self::Artifact {
//!         // Resolve ArtifactPromise to its artifact
//!         let leaf = resolver.resolve(&self.builder_leaf);
//!
//!         Node {
//!             leaf,
//!             value: resolver.get_my_state().copied().unwrap_or(42),
//!             // ...
//!         }
//!     }
//! }
//!
//! // The cache to storing already created artifacts
//! let mut cache = rc::ArtifactCache::new();
//!
//! // Constructing builders
//! let leaf_builder = rc::ArtifactPromise::new(BuilderLeaf::new());
//!
//! let node_builder_1 = ArtifactPromise::new(BuilderNode::new(leaf_builder.clone()));
//! let node_builder_2 = ArtifactPromise::new(BuilderNode::new(leaf_builder.clone()));
//!
//! // Using the cache to access the artifacts from the builders
//!
//! // The same builder results in same artifact
//! assert!(Rc::ptr_eq(&cache.get(&node_builder_1), &cache.get(&node_builder_1)));
//!
//! // Different builders result in different artifacts
//! assert!( ! Rc::ptr_eq(&cache.get(&node_builder_1), &cache.get(&node_builder_2)));
//!
//! // Different artifacts may link the same dependent artifact
//! assert!(Rc::ptr_eq(&cache.get(&node_builder_1).leaf, &cache.get(&node_builder_2).leaf));
//!
//! // Test dynamic state
//! assert_eq!(cache.get(&node_builder_1).value, 42);
//!
//! // Change state
//! cache.set_dyn_state(&node_builder_1, 127.into());
//! // Without invalidation, the cached artefact remains unchanged
//! assert_eq!(cache.get_dyn_state(&node_builder_1), Some(&mut 127));
//! assert_eq!(cache.get(&node_builder_1).value, 42);
//! // Invalidate node, and ensure it made use of the state
//! cache.invalidate(&node_builder_1);
//! assert_eq!(cache.get(&node_builder_1).value, 127);
//!
//! // State of node 2 remains unchanged
//! assert_eq!(cache.get(&node_builder_2).value, 42);
//! assert_eq!(cache.get_dyn_state(&node_builder_2), None);
//! ```
//!
//!
//!
//! ## Debugging
//!
//! `daab` comes with extensive debugging gear. However, in order to
//! keep the production impact as low as possible, the debugging facilities
//! are capsuled behind the **`diagnostics`** feature.
//!
//! Of course, the debugging feature is for the user of this crate to
//! debug their graphs. Therefore, it is rather modelled as a
//! diagnostics feature (hence the name). The diagnosis
//! is carried out by a [`Doctor`], which is a trait receiving various
//! internal events in order to record them, print them, or otherwise help
//! treating the bug.
//!
//! Care has been taken to keep the **`diagnostics`** feature broadly applicable
//! as well as keeping the non-`diagnostics` API compatible with the
//! `diagnostics`-API, meaning that a project not using the
//! `diagnostics` feature can be easily converted to using
//! `diagnostics`, usually by just replacing `ArtifactCache::new()`
//! with `ArtifactCache::new_with_doctor()`.
//! In order to store the `Doctor` the `ArtifactCache` is generic to a doctor,
//! which is important on its creation and for storing it by value.
//! The rest of the time the `ArtifactCache` uses `dyn Doctor` as its default
//! generic argument.
//! To ease conversion between them, all creatable `ArtifactCache`s
//! (i.e. not `ArtifactCache<dyn Doctor>`) implement `DerefMut` to
//! `&mut ArtifactCache<dyn Doctor>` which has all the important methods
//! implemented.
//!
//![`Doctor`]: diagnostics/trait.Doctor.html
//!
//!
//!
//! ## Features
//!
//! This crate offers the following features:
//!
//! - **`diagnostics`** enables elaborate graph and cache interaction debugging.
//!   It adds the `new_with_doctor()` function to the `ArtifactCache` and adds
//!   the `diagnostics` module with the `Doctor` trait definition and some
//!   default `Doctor`s.
//!
//! - **`tynm`** enable the optional dependency on the [`tynm`] crate which adds
//!   functionality to abbreviate type names, which are used by some default
//!   `Doctor`s, hence it is only useful in connection with the `diagnostics`
//!   feature.
//!
//![`tynm`]: https://crates.io/crates/tynm
//!

// prevents compilation with broken Deref impl causing nasty stack overflows.
#![deny(unconditional_recursion)]

#![warn(missing_docs)]


use std::collections::HashMap;
use std::collections::HashSet;
use std::any::Any;
use std::hash::Hash;
use std::hash::Hasher;
use std::fmt;
use std::fmt::Debug;
use std::borrow::Borrow;
use std::marker::PhantomData;

use cfg_if::cfg_if;

pub mod rc;
pub mod arc;
pub mod boxed;

pub mod canning;

pub mod utils;

use canning::CanBase;
use canning::Can;
use canning::CanOwned;
use canning::CanStrong;
use canning::CanSized;
use canning::CanRef;
use canning::CanRefMut;

cfg_if! {
	if #[cfg(feature = "unsized")] {
		use canning::CanUnsized;
	}
}

#[cfg(feature = "diagnostics")]
pub mod diagnostics;

cfg_if! {
	if #[cfg(feature = "diagnostics")] {
		use std::ops::Deref;
		use std::ops::DerefMut;
		use diagnostics::Doctor;
		use diagnostics::ArtifactHandle;
		use diagnostics::BuilderHandle;
		use diagnostics::NoopDoctor as DefDoctor;
	}
}


/// Represents a builder for an artifact.
///
/// Each builder is supposed to contain all direct dependencies such as other
/// builders.
/// In the `build()` function, `resolver` gives access to the `ArtifactCache`
/// in order to resolve depending builders (aka `ArtifactPromise`s) into their
/// respective artifacts.
///
pub trait Builder<ArtCan, BCan>: Debug
		where
			BCan: CanStrong {

	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;

	/// Type of the dynamic state of this builder.
	///
	/// The dynamic state can be used to store mutable data for the builder
	/// or to modify the builder for outside.
	///
	type DynState : Debug + 'static;

	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, cache: &mut ArtifactResolver<ArtCan, BCan, Self::DynState>) -> Self::Artifact;
}


pub type ArtifactPromise<B, BCan> = ArtifactPromiseSized<B, BCan>;


pub trait ArtifactPromiseTrait<B: ?Sized, BCan: Can<B>> {
	fn id(&self) -> BuilderId;

	fn accessor(&self) -> ArtifactPromiseAccessor<B>;

	fn as_can(&self) -> BCan;
}


/// Opaque artifact promise accessor, used internally.
pub struct ArtifactPromiseAccessor<'a, B: ?Sized> {
	builder: &'a B,
}


/// Encapsulates a `Builder` as promise for its artifact from the `ArtifactCache`.
///
/// This struct is essentially a wrapper around `Rc<B>`, but it provides a
/// `Hash` and `Eq` implementation based on the identity of the `Rc`s inner
/// value. In other words the address of the allocation behind the Rc is
/// compared instead of the semantics (also see [`Rc::ptr_eq()`]).
/// Thus all clones of an `ArtifactPromise` are considered identical.
///
/// An `ArtifactPromise` can be either resolved using the `ArtifactCache::get()`
/// or `ArtifactResolver::resolve()`, whatever is available.
///
/// [`Rc::ptr_eq()`]: https://doc.rust-lang.org/std/rc/struct.Rc.html#method.ptr_eq
///
pub struct ArtifactPromiseSized<B, BCan: Can<B>> {
	builder: BCan::Bin,
	_dummy: (),
}

impl<B, BCan: Can<B>> ArtifactPromiseSized<B, BCan> {
	/// Crates a new promise for the given builder.
	///
	pub fn new(builder: B) -> Self
			where
				BCan: CanSized<B>, {

		let bin = BCan::into_bin(builder);

		Self::new_binned(bin)
	}
}

impl<B, BCan: Can<B>> ArtifactPromiseSized<B, BCan> {
	/// Create a new promise for the given binned builder.
	///
	pub fn new_binned(builder_bin: BCan::Bin) -> Self {
		ArtifactPromiseSized {
			builder: builder_bin,
			_dummy: (),
		}
	}
}


impl<B, BCan: CanOwned<B>> ArtifactPromiseSized<B, BCan> {
	/// Returns the id of this artifact promise
	/// This Id has the following property:
	/// The ids of two artifact promises are the same if and only if
	/// they point to the same builder.
	pub fn id(&self) -> BuilderId {
		BuilderId::new(BCan::bin_as_ptr(&self.builder))
	}
}

impl<B, BCan: CanOwned<B>> ArtifactPromiseTrait<B, BCan> for ArtifactPromiseSized<B, BCan>
		where
			BCan::Bin: AsRef<B> + Clone, {

	fn id(&self) -> BuilderId {
		self.id()
	}

	fn accessor(&self) -> ArtifactPromiseAccessor<B> {
		ArtifactPromiseAccessor {
			builder: self.builder.as_ref(),
		}
	}

	fn as_can(&self) -> BCan {
		BCan::from_bin(self.builder.clone())
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		impl<B, BCan> ArtifactPromiseSized<B, BCan> where
				BCan: CanOwned<B>,
				BCan::Bin: Clone, {

			pub fn into_unsized<UB: ?Sized + 'static>(self) -> ArtifactPromiseUnsized<UB, BCan>
				where
					B: 'static + std::marker::Unsize<UB>,
					BCan: CanUnsized<B, UB> {

				ArtifactPromiseUnsized::new_binned(self.builder).into_unsized()
			}
		}
	}
}

impl<B, BCan: CanOwned<B>> Clone for ArtifactPromiseSized<B, BCan> where BCan::Bin: Clone {
	fn clone(&self) -> Self {
		ArtifactPromiseSized {
			builder: self.builder.clone(),
			_dummy: (),
		}
	}
}

impl<B, BCan: CanOwned<B>> Hash for ArtifactPromiseSized<B, BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl<B, BCan: CanOwned<B>> PartialEq for ArtifactPromiseSized<B, BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id().eq(&other.id())
	}
}

impl<B, BCan: CanOwned<B>> Eq for ArtifactPromiseSized<B, BCan> {
}

impl<B, BCan: CanOwned<B>> fmt::Pointer for ArtifactPromiseSized<B, BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", BCan::bin_as_ptr(&self.builder))
	}
}

impl<B, BCan: CanOwned<B>> fmt::Debug for ArtifactPromiseSized<B, BCan> where BCan::Bin: fmt::Debug {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "ArtifactPromise {{builder: {:?}, id: {:p}}}", self.builder, self.id())
	}
}

impl<B, BCan: CanSized<B>> From<B> for ArtifactPromiseSized<B, BCan> where BCan::Bin: fmt::Debug {
	fn from(builder: B) -> Self {
		ArtifactPromiseSized::new(builder)
	}
}


pub struct ArtifactPromiseUnsized<B: ?Sized, BCan: Can<B>> {
	builder: BCan::Bin,
	builder_canned: BCan,
	_dummy: (),
}

impl<B, BCan: CanOwned<B>> ArtifactPromiseUnsized<B, BCan> where BCan::Bin: Clone {
	/// Crates a new promise for the given builder.
	///
	pub fn new(builder: B) -> Self
			where
				BCan: CanSized<B>, {

		let bin = BCan::into_bin(builder);

		Self::new_binned(bin)
	}
}

impl<B: ?Sized, BCan: CanOwned<B>> ArtifactPromiseUnsized<B, BCan> where BCan::Bin: Clone {
	/// Create a new promise for the given binned builder.
	///
	pub fn new_binned(builder_bin: BCan::Bin) -> Self {
		ArtifactPromiseUnsized {
			builder: builder_bin.clone(),
			builder_canned: BCan::from_bin(builder_bin),
			_dummy: (),
		}
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		impl<B: ?Sized, BCan> ArtifactPromiseUnsized<B, BCan> where
				BCan: Can<B>, {

			pub fn into_unsized<UB: ?Sized + 'static>(self) -> ArtifactPromiseUnsized<UB, BCan>
				where
					B: 'static + std::marker::Unsize<UB>,
					BCan: CanUnsized<B, UB> {

				//let b: Rc<UB> = self.builder;

				ArtifactPromiseUnsized {
					builder: BCan::into_unsized(self.builder),
					builder_canned: self.builder_canned,
					_dummy: (),
				}
			}
		}
	}
}

impl<B: ?Sized, BCan: Can<B>> ArtifactPromiseUnsized<B, BCan> {
	/// Returns the id of this artifact promise
	/// This Id has the following property:
	/// The ids of two artifact promises are the same if and only if
	/// they point to the same builder.
	pub fn id(&self) -> BuilderId {
		BuilderId::new(BCan::can_as_ptr(&self.builder_canned))
	}

	pub fn as_ptr(&self) -> *const () {
		BCan::can_as_ptr(&self.builder_canned) as *const ()
	}

	pub fn new_from_clones(builder_bin: BCan::Bin, builder_can: BCan) -> Option<Self> {
		if BCan::bin_as_ptr(&builder_bin) == BCan::can_as_ptr(&builder_can) as *const () {
			Some(
				ArtifactPromiseUnsized {
					builder: builder_bin,
					builder_canned: builder_can,
					_dummy: (),
				}
			)
		} else {
			None
		}
	}
}


impl<B: ?Sized, BCan: Can<B>> ArtifactPromiseTrait<B, BCan> for ArtifactPromiseUnsized<B, BCan>
		where
			BCan::Bin: AsRef<B>,
			BCan: Clone, {

	fn id(&self) -> BuilderId {
		self.id()
	}

	fn accessor(&self) -> ArtifactPromiseAccessor<B> {
		ArtifactPromiseAccessor {
			builder: self.builder.as_ref(),
		}
	}

	fn as_can(&self) -> BCan {
		self.builder_canned.clone()
	}
}

impl<B: ?Sized, BCan: Can<B>> Clone for ArtifactPromiseUnsized<B, BCan> where BCan::Bin: Clone, BCan: Clone {
	fn clone(&self) -> Self {
		ArtifactPromiseUnsized {
			builder: self.builder.clone(),
			builder_canned: self.builder_canned.clone(),
			_dummy: (),
		}
	}
}



impl<B: ?Sized, BCan: Can<B>> Hash for ArtifactPromiseUnsized<B, BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl<B: ?Sized, BCan: Can<B>> PartialEq for ArtifactPromiseUnsized<B, BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id().eq(&other.id())
	}
}

impl<B: ?Sized, BCan: Can<B>> Eq for ArtifactPromiseUnsized<B, BCan> {
}

impl<B: ?Sized, BCan: Can<B>> fmt::Pointer for ArtifactPromiseUnsized<B, BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", BCan::can_as_ptr(&self.builder_canned))
	}
}

impl<B: ?Sized, BCan: Can<B>> fmt::Debug for ArtifactPromiseUnsized<B, BCan> where BCan::Bin: fmt::Debug {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "ArtifactPromiseUnsized {{builder: {:?}, id: {:p}}}", self.builder, self.id())
	}
}

impl<B, BCan: CanSized<B>> From<B> for ArtifactPromiseUnsized<B, BCan> where BCan::Bin: fmt::Debug {
	fn from(builder: B) -> Self {
		//ArtifactPromiseUnsized::new(builder)
		todo!()
	}
}




/// Resolves any `ArtifactPromise` to the artifact of the inner builder.
///
/// This struct is only available to `Builder`s within their `build()` method.
/// It gives certain access to the `ArtifactCache`, such as resolving
/// `ArtifactPromise`s.
///
/// The `ArtifactResolver` records each resolution of an `ArtifactPromise`
/// in order to keep track of dependencies between builders.
/// This dependency information is used for correct invalidation of dependants
/// on cache invalidation via `ArtifactCache::invalidate()`.
///
pub struct ArtifactResolver<'a, ArtCan, BCan: CanStrong, T = ()> {
	user: &'a BuilderEntry<BCan>,
	cache: &'a mut ArtifactCache<ArtCan, BCan>,
	#[cfg(feature = "diagnostics")]
	diag_builder: &'a BuilderHandle<BCan>,
	_b: PhantomData<T>,
}

impl<'a, ArtCan: Debug, BCan: CanStrong + Debug, T: 'static> ArtifactResolver<'a, ArtCan, BCan, T> {
	/// Resolves the given `ArtifactPromise` into its artifact either by
	/// looking up the cached value in the associated `ArtifactCache` or by
	/// building it.
	///
	pub fn resolve<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> ArtCan::Bin
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan> {

		cfg_if! {
			if #[cfg(feature = "diagnostics")] {
				self.cache.do_resolve(self.user, self.diag_builder, promise)
			} else {
				self.cache.do_resolve(self.user, promise)
			}
		}
	}

	/// Resolves the given `ArtifactPromise` into its artifact reference either
	/// by looking up the cached value in the associated `ArtifactCache` or by
	/// building it.
	///
	pub fn resolve_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> &B::Artifact
			where
				ArtCan: CanSized<B::Artifact> + CanRef<B::Artifact>,
				ArtCan::Bin: AsRef<B::Artifact>,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {

		cfg_if! {
			if #[cfg(feature = "diagnostics")] {
				self.cache.do_resolve_ref(self.user, self.diag_builder, promise)
			} else {
				self.cache.do_resolve_ref(self.user, promise)
			}
		}
	}

	/// Resolves the given `ArtifactPromise` into a clone of its artifact by
	/// using `resolve_ref()` and `clone().
	///
	pub fn resolve_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			builder: &AP
		) -> B::Artifact
			where
				ArtCan: CanSized<B::Artifact> + CanRef<B::Artifact>,
				ArtCan::Bin: AsRef<B::Artifact>,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				B::Artifact: Clone,
				AP: ArtifactPromiseTrait<B, BCan>  {


		// Get the artifact by ref and clone it
		self.resolve_ref(builder).clone()
	}


	// TODO: consider whether mutable access is actually a good option
	// TODO: consider may be to even allow invalidation


	/// Returns the dynamic state of this builder.
	///
	/// ## Panic
	///
	/// This function panics if no dynamic state has been set for this builder.
	///
	pub fn my_state(&mut self) -> &mut T {
		self.cache.get_dyn_state_cast(self.user.borrow()).unwrap()
	}

	/// Gets the dynamic state of the given builder.
	///
	pub fn get_my_state(&mut self) -> Option<&mut T> {
		self.cache.get_dyn_state_cast(self.user.borrow())
	}

	/// Get and cast the dynamic static of given builder id.
	///
	/// `T` must be the type of the respective dynamic state of `bid`, or panics.
	///
	pub fn get_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
		&mut self,
		promise: &AP
	) -> Option<&mut B::DynState>
			where
				BCan: Can<B>,
				ArtCan: Can<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>, {


		self.cache.get_dyn_state(promise)
	}
}


/// Id to differentiate builder instances across types.
///
/// Notice, this type simply wraps `*const` to the builder `Rc`s.
/// Consequentially, a `BuilderId`s validity is limited to the life time of
/// the respective `Builder`.
///
#[derive(Clone, Debug, Copy, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuilderId(*const ());

// Requires Send&Sync for Arc. This safe because the pointer is never
// dereference and only used for Hash and Eq.
//
// The unsafe could be eliminated by storing just an integer casted from the
// pointer, but currently (1.40) it does not seem posible to cast an dyn ptr
// to any int.
unsafe impl Send for BuilderId {}
unsafe impl Sync for BuilderId {}

impl BuilderId {
	fn new(ptr: *const dyn Any) -> Self {
		BuilderId(ptr as *const ())
	}
}

impl fmt::Pointer for BuilderId {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		fmt::Pointer::fmt(&self.0, fmt)
	}
}

/*
impl Borrow<BuilderIdRaw> for BuilderId {
	fn borrow(&self) -> &BuilderIdRaw {
		&self.0
	}
}
*/

type BuilderIdRaw = *const dyn Any;


/// Auxiliary struct fro the `ArtifactCache` containing an untyped (aka
/// `dyn Any`) ArtifactPromise.
///
#[derive(Clone, Debug)]
pub struct BuilderEntry<BCan> {
	builder: BCan,
	id: BuilderId,
}

impl<BCan> BuilderEntry<BCan> {
	fn new<B: ?Sized + 'static>(value: BCan) -> Self
			where BCan: Can<B> {

		let id = BuilderId::new(value.can_as_ptr());

		BuilderEntry {
			builder: value,
			id,
		}
	}
}

impl<BCan> Hash for BuilderEntry<BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

impl<BCan> PartialEq for BuilderEntry<BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id.eq(&other.id)
	}
}

impl<BCan> Eq for BuilderEntry<BCan> {
}

impl<BCan> Borrow<BuilderId> for BuilderEntry<BCan> {
	fn borrow(&self) -> &BuilderId {
		&self.id
	}
}

impl<BCan: CanBase> fmt::Pointer for BuilderEntry<BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", self.builder.can_as_ptr())
	}
}



/// Structure for caching and looking up artifacts.
///
/// The `ArtifactCache` is the central structure of this crate. It helps to
/// avoid dependency duplication when multiple `Builder`s depend on the same
/// artifact.
///
/// Since all `Builder`s in the context of this crate are supposed to be wrapped
/// within `ArtifactPromise`s, the `ArtifactCache` is the only way of acquiring
/// an artifact in the first place.
///
/// Notice In the debugging version (when the **`diagnostics`** feature is active),
/// this struct contains a debuger `Doctor`, which
/// allows run-time inspection of various events.
/// In order to store it, the **`diagnostics`** `ArtifactCache` is generic to
/// some `Doctor`.
/// The `new()` method then returns a `ArtifactCache<NoopDoctor>`
/// and `new_with_doctor()` returns some `ArtifactCache<T>`.
///
/// Only an `ArtifactCache<T>` with `T: Sized` can be store in variables.
/// However, since most of the code does not care about the concrete
/// `Doctor` the default generic is `dyn Doctor`, on which all other methods are
/// defined.
/// An `ArtifactCache<dyn Doctor>` can not be stored, but it can be passed
/// on by reference (e.g. as `&mut ArtifactCache`). This prevents the use of
/// additional generics in **`diagnostics`** mode, and allows to easier achive
/// compatibility between **`diagnostics`** and non-**`diagnostics`** mode.
/// To ease conversion between `ArtifactCache<T>` and
/// `ArtifactCache<dyn Doctor>` (aka `ArtifactCache`), all creatable
/// `ArtifactCache`s (i.e. not `ArtifactCache<dyn Doctor>`) implement `DerefMut`
/// to `ArtifactCache<dyn Doctor>`.
///
pub struct ArtifactCache<ArtCan, BCan, #[cfg(feature = "diagnostics")] T: ?Sized = dyn Doctor<ArtCan, BCan>> where BCan: CanStrong {
	/// Maps Builder-Capsules to their Artifact value
	cache: HashMap<BuilderId, ArtCan>,

	/// Maps Builder-Capsules to their DynState value
	dyn_state: HashMap<BuilderId, Box<dyn Any>>,

	/// Tracks the direct promise dependants of each promise
	dependants: HashMap<BuilderId, HashSet<BuilderId>>,

	/// Keeps a weak reference to all known builder ids that are those used in
	/// `cache` and/or dyn_state.
	know_builders: HashMap<BuilderId, <BCan as CanStrong>::CanWeak>,

	/// The doctor for error diagnostics.
	#[cfg(feature = "diagnostics")]
	doctor: T,
}

cfg_if! {
	if #[cfg(feature = "diagnostics")] {
		impl<ArtCan, BCan: CanStrong> Default for ArtifactCache<ArtCan, BCan, DefDoctor> {
			fn default() -> Self {
				ArtifactCache::new()
			}
		}

		type ArtifactCacheOwned<ArtCan, BCan> = ArtifactCache<ArtCan, BCan, DefDoctor>;

		impl<ArtCan: Debug, BCan: CanStrong + Debug, T: Debug> Debug for ArtifactCache<ArtCan, BCan, T> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "ArtifactCache {{ cache: {:?}, dependants: {:?}, doctor: {:?} }}",
					self.cache, self.dependants, self.doctor)
			}
		}

		impl<ArtCan, BCan: CanStrong> ArtifactCache<ArtCan, BCan, DefDoctor> {
			/// Creates a new empty cache with a dummy doctor.
			///
			pub fn new() -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependants: HashMap::new(),
					know_builders: HashMap::new(),

					doctor: DefDoctor::default(),
				}
			}
		}

		impl<ArtCan, BCan: CanStrong, T: Doctor<ArtCan, BCan> + 'static> ArtifactCache<ArtCan, BCan, T> {

			/// Creates new empty cache with given doctor for inspection.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn new_with_doctor(doctor: T) -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependants: HashMap::new(),
					know_builders: HashMap::new(),

					doctor,
				}
			}

			/// Returns a reference of the inner doctor.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn get_doctor(&mut self) -> &mut T {
				&mut self.doctor
			}

			/// Consumes the `ArtifactCache` and returns the inner doctor.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn into_doctor(self) -> T {
				self.doctor
			}
		}

		impl<ArtCan, BCan: CanStrong, T: Doctor<ArtCan, BCan> + 'static> Deref for ArtifactCache<ArtCan, BCan, T> {
			type Target = ArtifactCache<ArtCan, BCan>;

			fn deref(&self) -> &Self::Target {
				self
			}
		}

		impl<ArtCan, BCan: CanStrong, T: Doctor<ArtCan, BCan> + 'static> DerefMut for ArtifactCache<ArtCan, BCan, T> {
			fn deref_mut(&mut self) -> &mut Self::Target {
				self
			}
		}


	} else {
		impl<ArtCan, BCan: CanStrong> Default for ArtifactCache<ArtCan, BCan> {
			fn default() -> Self {
				ArtifactCache::new()
			}
		}

		type ArtifactCacheOwned<ArtCan, BCan> = ArtifactCache<ArtCan, BCan>;

		impl<ArtCan: Debug, BCan: CanStrong + Debug> Debug for ArtifactCache<ArtCan, BCan> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "ArtifactCache {{ cache: {:?}, dependants: {:?} }}",
					self.cache, self.dependants)
			}
		}

		impl<ArtCan, BCan: CanStrong> ArtifactCache<ArtCan, BCan> {
			/// Creates a new empty cache.
			///
			pub fn new() -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependants: HashMap::new(),
					know_builders: HashMap::new(),
				}
			}
		}
	}
}

/// Auxiliarry function to casts an `Option` of `Box` of `Any` to `T`.
///
/// Must only be used with the correct `T`, or panics.
///
fn cast_dyn_state<T: 'static>(v: Option<Box<dyn Any>>) -> Option<Box<T>> {
	v.map(
		|b| {
			// Ensure value type
			b.downcast()
				.expect("Cached Builder DynState is of invalid type")
		}
	)
}

impl<ArtCan: Debug, BCan: CanStrong + Debug> ArtifactCache<ArtCan, BCan> {

	/// Resolves the artifact of `promise` and records dependency between `user`
	/// and `promise`.
	///
	fn do_resolve<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			user: &BuilderEntry<BCan>,
			#[cfg(feature = "diagnostics")]
			diag_builder: &BuilderHandle<BCan>,
			promise: &AP
		) -> ArtCan::Bin
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		let deps = self.get_dependants(promise);
		if !deps.contains(&user.id) {
			deps.insert(*user.borrow());
		}

		#[cfg(feature = "diagnostics")]
		self.doctor.resolve(diag_builder, &BuilderHandle::new(promise));

		self.get(promise)
	}

	/// Resolves the artifact reference of `promise` and records dependency between `user`
	/// and `promise`.
	///
	fn do_resolve_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			user: &BuilderEntry<BCan>,
			#[cfg(feature = "diagnostics")]
			diag_builder: &BuilderHandle<BCan>,
			promise: &AP
		) -> &B::Artifact
			where
				ArtCan: CanSized<B::Artifact> + CanRef<B::Artifact>,
				ArtCan::Bin: AsRef<B::Artifact>,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan>,  {


		let deps = self.get_dependants(promise);
		if !deps.contains(&user.id) {
			deps.insert(*user.borrow());
		}

		#[cfg(feature = "diagnostics")]
		self.doctor.resolve(diag_builder, &BuilderHandle::new(promise.clone()));

		self.get_ref(promise)
	}

	/// Returns the vector of dependants of promise
	///
	fn get_dependants<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> &mut HashSet<BuilderId>
			where
				ArtCan: Can<B::Artifact>,
				BCan: Can<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		if !self.dependants.contains_key(&promise.id()) {
			self.dependants.insert(promise.id(), HashSet::new());
		}

		self.dependants.get_mut(&promise.id()).unwrap()
	}

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			builder: &AP
		) -> Option<ArtCan::Bin>
			where
				ArtCan: CanOwned<B::Artifact>,
				ArtCan: Clone,
				BCan: Can<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.cache.get(&builder.id()).map(
			|ent| {
				// Ensure value type
				ent.clone().downcast_can()
					.expect("Cached Builder Artifact is of invalid type")
			}
		)
	}

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			builder: &AP
		) -> Option<&B::Artifact>
			where
				ArtCan: CanRef<B::Artifact>,
				BCan: Can<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.cache.get(&builder.id()).map(
			|ent| {
				// Ensure value type
				ent.downcast_can_ref()
					.expect("Cached Builder Artifact is of invalid type")
			}
		)
	}

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			builder: &AP
		) -> Option<&mut B::Artifact>
			where
				ArtCan: CanRefMut<B::Artifact>,
				BCan: Can<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.cache.get_mut(&builder.id()).map(
			|ent| {
				// Ensure value type
				ent.downcast_can_mut()
					.expect("Cached Builder Artifact is of invalid type")
			}
		)
	}

	/// Get and cast a clone of the stored artifact if it exists.
	///
	pub fn lookup_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			builder: &AP
		) -> Option<B::Artifact>
			where
				ArtCan: CanRef<B::Artifact>,
				BCan: Can<B>,
				B::Artifact: Clone,
				AP: ArtifactPromiseTrait<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.lookup_ref(builder).cloned()
	}

	/// Build and insert the artifact for `promise`.
	///
	fn build<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> &mut ArtCan
			where
				ArtCan: CanSized<B::Artifact>,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		self.make_builder_known(promise);

		let ent = BuilderEntry::new(promise.as_can());

		#[cfg(feature = "diagnostics")]
		let diag_builder = BuilderHandle::new(promise.clone());

		let art = promise.accessor().builder.build(
			&mut ArtifactResolver {
				user: &ent,
				cache: self,
				#[cfg(feature = "diagnostics")]
				diag_builder: &diag_builder,
				_b: PhantomData,
			},
		);

		let art_bin = ArtCan::into_bin(art);

		cfg_if!(
			if #[cfg(feature = "diagnostics")] {
				let handle = ArtifactHandle::new(art_bin);

				// Update doctor on diagnostics mode
				self.doctor.build(&diag_builder, &handle);

				let art_can = handle.into_inner();
			} else {
				let art_can = ArtCan::from_bin(art_bin);
			}
		);

		//let art_can = ArtCan::from_bin(art_bin);


		// keep id
		let id = promise.id();

		// Insert artifact
		self.cache.insert(
			id,
			art_can,
		);

		// Just unwrap, since we just inserted it
		self.cache.get_mut(&id).unwrap()

	}

	/// Gets the artifact of the given builder.
	///
	/// This method looks up whether the artifact for the given builder is still
	/// present in the cache, or it will use the builder to build a fresh
	/// artifact and store it in the cache for later reuse.
	///
	/// Notice the given promise as well as the artifact will be stored in the
	/// cache, until `clear()` or `invalidate()` with that promise are called.
	/// The promise is kept to prevent it from deallocating, which is important
	/// for correctness of the pointer comparison, internally done by the
	/// promise.
	///
	///	If all strong references of the respective builder are out of scope, the
	/// `garbage_collection()` method can be used to get rid of the cached
	/// promise including the possibly still cached artifact and dyn state.
	///
	pub fn get<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> ArtCan::Bin
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		if let Some(art) = self.lookup(promise) {
			art

		} else {
			self.build(promise).clone().downcast_can()
				.expect("Cached Builder Artifact is of invalid type")
		}
	}

	/// Gets a reference of the artifact of the given builder.
	///
	pub fn get_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> &B::Artifact
			where
				ArtCan: CanSized<B::Artifact> + CanRef<B::Artifact>,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		if self.lookup_ref(promise).is_some() {
			self.lookup_ref(promise).unwrap()

		} else {
			self.build(promise).downcast_can_ref()
				.expect("Cached Builder Artifact is of invalid type")
		}
	}

	/// Gets a mutable reference of the artifact of the given builder.
	///
	pub fn get_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> &mut B::Artifact
			where
				ArtCan: CanSized<B::Artifact> + CanRefMut<B::Artifact>,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		if self.lookup_mut(promise).is_some() {
			// Here, requires a second look up because due to the build in the
			// else case, an `if let Some(_)` won't work due to lifetiem issues
			self.lookup_mut(promise).unwrap()

		} else {
			self.build(promise).downcast_can_mut()
				.expect("Cached Builder Artifact is of invalid type")
		}
	}

	/// Get a clone of the artifact of the given builder.
	///
	pub fn get_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> B::Artifact
			where
				ArtCan: CanSized<B::Artifact> + CanRef<B::Artifact>,
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B>,
				B::Artifact: Clone,
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.get_ref(promise).clone()
	}


	/// Get and cast the dynamic static of given builder id.
	///
	/// `T` must be the type of the respective dynamic state of `bid`, or panics.
	///
	fn get_dyn_state_cast<T: 'static>(&mut self, bid: &BuilderId) -> Option<&mut T> {

		self.dyn_state.get_mut(bid)
		.map(
			|b| {
				// Ensure state type
				b.downcast_mut()
					.expect("Cached Builder DynState is of invalid type")
			}
		)
	}

	/// Gets the dynamic state of the given builder.
	///
	pub fn get_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self, promise: &AP
		) -> Option<&mut B::DynState>
			where
				BCan: Can<B>,
				ArtCan: Can<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		self.get_dyn_state_cast(&promise.id())
	}

	/// Sets the dynamic state of the given builder.
	///
	pub fn set_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP,
			user_data: B::DynState
		) -> Option<Box<B::DynState>>
			where
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				ArtCan: Can<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.make_builder_known(promise);

		cast_dyn_state(
			self.dyn_state.insert(promise.id(), Box::new(user_data))
		)
	}

	// TODO: add convenience function such as:
	// pub fn set_user_data_and_invalidate_on_change(...)

	/// Deletes the dynamic state of the given builder.
	///
	pub fn remove_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Option<Box<B::DynState>>
			where
				BCan: Can<B>,
				ArtCan: Can<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {

		let bid = promise.id();

		// Remove weak reference if no builder exists
		if !self.cache.contains_key(&bid) {
			self.know_builders.remove(&bid);
		}

		cast_dyn_state(
			self.dyn_state.remove(&bid)
		)
	}

	/// Deletes all dynamic states of this cache.
	///
	pub fn clear_dyn_state(&mut self) {
		self.dyn_state.clear();

		// Remove weak reference for those without builder
		self.cleanup_unused_weak_refs();
	}

	/// Clears the entire cache including all kept promise, artifacts and
	/// dynamic states.
	///
	pub fn clear(&mut self) {
		self.cache.clear();
		self.dyn_state.clear();
		self.dependants.clear();
		self.know_builders.clear();

		#[cfg(feature = "diagnostics")]
		self.doctor.clear();
	}

	/// Auxiliary invalidation function using an untyped (aka `dyn Any`) `BuilderId`.
	///
	fn invalidate_any(&mut self, builder: &BuilderId) {
		// TODO could be done in a iterative loop instead of recursion
		// However, this would be more significant, if the building would be
		// done in a loop too.

		if let Some(set) = self.dependants.remove(builder) {
			for dep in set {
				self.invalidate_any(&dep);
			}
		}

		self.cache.remove(builder);
	}

	/// Removes the given promise with its cached artifact from the cache and
	/// all depending artifacts (with their promises).
	///
	/// Depending artifacts are all artifacts which used the former during
	/// its building. The dependencies are automatically tracked via the
	/// `ArtifactResolver`.
	///
	pub fn invalidate<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		)
			where
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				ArtCan: Can<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {


		self.invalidate_any(&promise.id());

		#[cfg(feature = "diagnostics")]
		self.doctor.invalidate(&BuilderHandle::new(promise));

		// Remove weak reference for those without dyn_state
		self.cleanup_unused_weak_refs();
	}

	/// Invalidates all builders and their dyn state which can not be builded
	/// any more, because there are no more references to them.
	///
	/// This function has the complexity of O(n) with n being the number of
	/// used builders. Thus this function is not light weight but should be
	/// called regularly at appropriate locations, i.e. where many builders
	/// go out of scope.
	///
	/// If builders and dynamic states are explicitly invalidated and removed
	/// before going out of scope, this function has no effect.
	///
	/// Notice, that under normal circumstances this function only cleans up
	/// old builders which can not be used any more by the user. However, it is
	/// possible to create dependent builders without the dependent holding a
	/// strong reference to its dependency. Is such a case the dependent would
	/// get invalidate too nonetheless it might still be used. Therefor, any
	/// dependent builder should hold a strong reference to its builder.
	///
	pub fn garbage_collection(&mut self) {
		let unreachable_builder_ids: Vec<_> = self.know_builders.iter()
			.filter(|(_bid, weak)| BCan::upgrade_from_weak(&weak).is_none())
			.map(|(bid, _weak)| *bid)
			.collect();

		for bid in unreachable_builder_ids {
			self.invalidate_any(&bid);
			self.dyn_state.remove(&bid);
			self.know_builders.remove(&bid);
		}
	}

	/// Remove any weak builder reference that is no longer used.
	fn cleanup_unused_weak_refs(&mut self) {
		let unused_builder_ids: Vec<_> = self.know_builders.keys().filter(|b|
			!(self.cache.contains_key(*b) || self.dyn_state.contains_key(*b))
		).cloned().collect();

		for bid in unused_builder_ids {
			self.know_builders.remove(&bid);
		}
	}

	/// Enlist given builder as known builder, that is to keep its weak
	/// reference while it is used in `cache` or `dyn_state`.
	fn make_builder_known<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		)
			where
				BCan: Can<B> + Clone,
				BCan::Bin: Clone,
				ArtCan: Can<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {

		let bid = promise.id();

		if !self.know_builders.contains_key(&bid) {
			self.know_builders.insert(bid, promise.as_can().clone().downgrade());
		}
	}

	/// Returns the number of currently kept artifact promises.
	///
	/// This method is offered as kind of debugging or analysis tool for
	/// keeping track about the number of active builders.
	///
	/// When adding dynamic state or issuing the building of a promise may
	/// increase the returned number. Like wise the invalidation and removal of
	/// dynamic state might decrement this count. Additionally, if there are
	/// no more artifact promises to a used builder, the `garbage_collection`
	/// method might also reduce this number.
	///
	pub fn number_of_known_builders(&self) -> usize {
		self.know_builders.len()
	}
}





// -----------

#[cfg(test)]
mod test;

#[cfg(test)]
mod multi_level_test;





