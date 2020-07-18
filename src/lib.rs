

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
//! `Builder`s are supposed to be wrapped in [`Blueprint`]s, which prevents
//! to call its `Builder::build()` method directly. In other respects, the
//! `Blueprint` acts a lot like an `Rc` and thus allows to share one
//! instance among several dependants.
//! This `Rc`-like structure creates naturally a DAG.
//!
//! For building a `Builder`s artifact, its `Builder::build()` method is
//! provided with a [`Resolver`] that allows to resolve depending
//! `Blueprint`s into their respective artifacts, which is,
//! in order to form a DAG, wrapped behind a `Rc`.
//!
//! As entry point serves the [`Cache`], which allows outside of a
//! `Builder` to resolve any `Blueprint` to its artifact. The
//! `Cache` is essentially a cache for artifacts. It can be used to
//! translate any number of `Blueprint`s to their respective artifact,
//! while sharing their common dependencies.
//! Consequently, resolving the same `Blueprint` using the same
//! `Cache` results in the same `Rc`ed artifact.
//! However, using different `Cache`s results in different artifacts.
//!
//! The `Cache` has a `clear()` method to reset the cache.
//! This could be useful to free the resources kept by all artifacts and
//! builders, which are cached in it, or when artifacts shall be explicitly
//! recreated, e.g. to form a second independent artifact DAG.
//! Additionally, `Cache` has an `invalidate()` method to remove a single
//! builder and artifact including its dependants (i.e. those artifacts which had
//! used the invalidated one).
//!
//![`Builder`]: trait.Builder.html
//![`Blueprint`]: struct.Blueprint.html
//![`Resolver`]: struct.Resolver.html
//![`Cache`]: struct.Cache.html
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
//!     fn build(&self, _resolver: &mut rc::Resolver) -> Self::Artifact {
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
//!     builder_leaf: rc::Blueprint<BuilderLeaf>, // Dependency builder
//!     // ...
//! }
//! impl BuilderNode {
//!     pub fn new(builder_leaf: rc::Blueprint<BuilderLeaf>) -> Self {
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
//!     fn build(&self, resolver: &mut rc::Resolver<Self::DynState>) -> Self::Artifact {
//!         // Resolve Blueprint to its artifact
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
//! let mut cache = rc::Cache::new();
//!
//! // Constructing builders
//! let leaf_builder = rc::Blueprint::new(BuilderLeaf::new());
//!
//! let node_builder_1 = Blueprint::new(BuilderNode::new(leaf_builder.clone()));
//! let node_builder_2 = Blueprint::new(BuilderNode::new(leaf_builder.clone()));
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
//! assert_eq!(cache.get_dyn_state(&node_builder_1), Some(& 127));
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
//! `diagnostics`, usually by just replacing `Cache::new()`
//! with `Cache::new_with_doctor()`.
//! In order to store the `Doctor` the `Cache` is generic to a doctor,
//! which is important on its creation and for storing it by value.
//! The rest of the time the `Cache` uses `dyn Doctor` as its default
//! generic argument.
//! To ease conversion between them, all creatable `Cache`s
//! (i.e. not `Cache<dyn Doctor>`) implement `DerefMut` to
//! `&mut Cache<dyn Doctor>` which has all the important methods
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
//!   It adds the `new_with_doctor()` function to the `Cache` and adds
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


use std::any::Any;
use std::hash::Hash;
use std::fmt;
use std::fmt::Debug;

use cfg_if::cfg_if;

pub mod rc;
pub mod arc;
pub mod boxed;

pub mod blueprint;
pub mod canning;
pub mod cache;
pub mod utils;

use canning::Can;
use canning::CanStrong;
use canning::CanBuilder;
use canning::CanSized;
use canning::CanRef;
use canning::CanRefMut;

pub use blueprint::Promise;
pub use blueprint::Blueprint;
pub use blueprint::BlueprintUnsized;
pub use cache::Cache;
pub use cache::CacheOwned;
pub use cache::Resolver;

cfg_if! {
	if #[cfg(feature = "unsized")] {
		use canning::CanUnsized;
	}
}

#[cfg(feature = "diagnostics")]
pub mod diagnostics;

cfg_if! {
	if #[cfg(feature = "diagnostics")] {
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
/// In the `build()` function, `resolver` gives access to the `Cache`
/// in order to resolve depending builders (aka `Blueprint`s) into their
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

	/// Produces an artifact using the given `Resolver` for resolving
	/// dependencies.
	///
	fn build(&self, cache: &mut Resolver<ArtCan, BCan, Self::DynState>) -> Self::Artifact;
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

	fn as_ptr(&self) -> *const () {
		self.0
	}
}

impl fmt::Pointer for BuilderId {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		fmt::Pointer::fmt(&self.0, fmt)
	}
}



// -----------

#[cfg(test)]
mod test;

#[cfg(test)]
mod multi_level_test;





