

#![cfg_attr(feature = "unsized", feature(unsize))]

//!
//! DAG Aware Artifact Builder
//! ==========================
//!
//! Rust crate for managing the building and caching of artifacts which are
//! connected in a directed acyclic graph (DAG) like manner, i.e. artifacts may
//! depend on others.
//!
//! The caching provided by this crate could be especially useful if the
//! artifact builders use consumable resources, the building process is a
//! heavyweight procedure, or a given DAG dependency structure among the
//! builders shall be properly preserved among their artifacts.
//!
//! Minimal Rust version: **1.40**
//!
//!
//!
//! ## Basic Concept
//!
//! The basic concept of daab revolves around Builders, which are user provided
//! structs that implement the [`Builder`] trait. That trait essentially has an
//! associated type [`Artifact`] and method [`build`] where the latter will
//! produce a value of the `Artifact` type, which will be subsequently be
//! referred to as Artifact. In order to be able to depend on the Artifact of
//! other Builders, the `build` method also gets a [`Resolver`] that allows
//! to retrieve the Artifacts of others.
//!
//! In order to allow Builders and Artifacts to form a directed acyclic graph
//! thi crate provides at its heart a Artifact [`Cache`] which keeps the
//! Artifacts of Builders in order to prevent the Builders to produce multiple
//! equal Artifacts. Thus different Builders may depend on same Builder and
//! getting the same Artifact from the `Cache`.
//!
//! To be able to share Builders and Artifacts this crate also provides a
//! concept of Cans and Bins, which in the most basic case are simply an opaque
//! `Rc<dyn Any>` and a transparent `Rc<T>`, respectively. These are referred to
//! by the generic arguments of e.g. the `Cache`. For more details consult the
//! [`canning`] module.
//!
//! Additional to the canning, the `Cache` expects Builders to wrapped in a
//! opaque [`Blueprint`] enforcing encapsulation, i.e. it prevents users from
//! accessing the inner struct (the one which implements the `Builder` trait),
//! while only allowing the `Cache` itself to call its `build` method.
//!
//!
//!
//! ### Getting started
//!
//! For basic concept (explained above) there exists simplified traits which skip over the more
//! advanced features. One such simplified trait is the [`SimpleBuilder`] of the
//! [`rc`] module, which uses `Rc`s for canning and has simplified aliases
//! (minimal generic arguments) for all the above types. For getting started
//! that `rc` module is probably the best place to start.
//!
//!
//!
//! ## Detailed Concept
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
//![`Artifact`]: trait.Builder.html#associatedtype.Artifact
//![`build`]: trait.Builder.html#tymethod.build
//![`SimpleBuilder`]: rc/trait.SimpleBuilder.html
//![`rc`]: rc/index.html
//![`canning`]: canning/index.html
//![`Blueprint`]: blueprint/struct.Blueprint.html
//![`Resolver`]: cache/struct.Resolver.html
//![`Cache`]: cache/struct.Cache.html
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
//!     type Err = Never;
//!
//!     fn build(&self, resolver: &mut rc::Resolver<Self::DynState>) -> Result<Self::Artifact, Never> {
//!         // Resolve Blueprint to its artifact
//!         // Unpacking because the Err type is Never.
//!         let leaf = resolver.resolve(&self.builder_leaf).unpack();
//!
//!         Ok(Node {
//!             leaf,
//!             value: *resolver.my_state(),
//!             // ...
//!         })
//!     }
//!     fn init_dyn_state(&self) -> Self::DynState {
//!         42
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
//! assert!(Rc::ptr_eq(&cache.get(&node_builder_1).unpack(), &cache.get(&node_builder_1).unpack()));
//!
//! // Different builders result in different artifacts
//! assert!( ! Rc::ptr_eq(&cache.get(&node_builder_1).unpack(), &cache.get(&node_builder_2).unpack()));
//!
//! // Different artifacts may link the same dependent artifact
//! assert!(Rc::ptr_eq(&cache.get(&node_builder_1).unpack().leaf, &cache.get(&node_builder_2).unpack().leaf));
//!
//! // Purge builder 2 to ensure the following does not affect it
//! cache.purge(&node_builder_2);
//!
//! // Test dynamic state
//! assert_eq!(cache.get(&node_builder_1).unpack().value, 42);
//!
//! // Change state
//! *cache.dyn_state_mut(&node_builder_1) = 127.into();
//! // Without invalidation, the cached artefact remains unchanged
//! assert_eq!(cache.dyn_state(&node_builder_1), &127);
//! // Invalidate node, and ensure it made use of the state
//! assert_eq!(cache.get(&node_builder_1).unpack().value, 127);
//!
//! // State of node 2 remains unchanged
//! assert_eq!(cache.get_dyn_state(&node_builder_2), None);
//! assert_eq!(cache.get(&node_builder_2).unpack().value, 42);
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
//! - **`unsized`** enables better conversion between unsized Builders with
//!   [`BlueprintUnsized::into_unsized`]. **This feature requires Nightly
//!   Rust**.
//!
//![`tynm`]: https://crates.io/crates/tynm
//![`BlueprintUnsized::into_unsized`]: blueprint/struct.BlueprintUnsized.html#method.into_unsized
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

/// `daab`s Prelude.
///
/// This module contains some traits which are useful to be in scope. Just
/// write in your code:
///
/// ```rust
/// use daab::prelude::*;
/// ```
///
pub mod prelude {
	pub use crate::Unpacking;
}


// Pub-use the Never type of the never crate.
pub use never::Never;

/// Unpack a result into its value.
fn unpack<T>(res: Result<T,Never>) -> T {
    match res {
        Ok(t) => t,
        Err(n) => match n {},
    }
}

/// Unpacking a composite type into its inner value.
///
/// This trait is use in contexts where `Never` appears. For instance this
/// trait is implemented on `Result<T,Never>` to unpack `T`, which is its only
/// value as `Never` is uninhabited i.e. can never exist.
///
/// One can think about unpacking as a compile-time guaranteed non-panicking
/// alternative to unwrapping. Therefore, if unpacking is available it should
/// be preferred over unwrapping.
///
pub trait Unpacking {
	/// The type to be unpacked.
    type Inner;

	/// Unpacking into its inner value.
	///
	/// This function guarantees to never fail nor panic.
	///
    fn unpack(self) -> Self::Inner;
}
impl<T> Unpacking for Result<T,Never> {
    type Inner = T;

    fn unpack(self) -> T {
        unpack(self)
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
pub trait Builder<ArtCan, BCan>: Debug + 'static
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

	/// Error type returned by this Builder in case of failure to produce an
	/// Artifact.
	type Err : Debug + 'static;

	/// Produces an artifact using the given `Resolver` for resolving
	/// dependencies.
	///
	fn build(&self, cache: &mut Resolver<ArtCan, BCan, Self::DynState>)
		-> Result<Self::Artifact, Self::Err>;

	/// Return an inital dynamic state for this builder.
	/// 
	/// When a builder is first seen by a `Cache` the cache will use this method
	/// to obtain an inital value for the dynamic state of this builder.
	/// 
	fn init_dyn_state(&self) -> Self::DynState;
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





