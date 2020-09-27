// Only warn about unsafe code in general (needed for some tests)
#![warn(unsafe_code)]
// If not in test mode, forbid it entirely!
#![cfg_attr(not(test), forbid(unsafe_code))]

// Enables casting of trait-objects behind a Can
#![cfg_attr(feature = "unsized", feature(unsize))]

// Enable annotating features requirements in docs
#![cfg_attr(feature = "doc_cfg", feature(doc_cfg))]

// prevents compilation with broken Deref impl causing nasty stack overflows.
#![deny(unconditional_recursion)]

// Ensures that `pub` means published in the public API.
// This property is useful for reasoning about breaking API changes.
#![deny(unreachable_pub)]

// Prevents public API entries without a doc comment.
#![warn(missing_docs)]


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
//! The basic concept of daab revolves around _Builders_, which are user provided
//! structs that implement the [`Builder`] trait. That trait essentially has an
//! associated type [`Artifact`] and method [`build`] where the latter will
//! produce a value of the `Artifact` type, which will be subsequently be
//! referred to as _Artifact_. In order to be able to depend on the Artifact of
//! other Builders, the `build` method also gets a [`Resolver`] that allows
//! to retrieve the Artifacts of others.
//!
//! In order to allow Builders and Artifacts to form a directed acyclic graph
//! this crate provides at its heart an Artifact [`Cache`] which keeps the
//! Artifacts of Builders in order to prevent the Builders to produce multiple
//! equal Artifacts. Thus different Builders may depend on same Builder and
//! getting the same Artifact from the `Cache`.
//!
//! To be able to share Builders and Artifacts this crate also provides a
//! concept of _Cans_ and _Bins_, which in the most basic case are simply an opaque
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
//! For the basic concept (explained above) there exists simplified traits
//! which skip over the more
//! advanced features. One such simplified trait is the [`SimpleBuilder`] of the
//! [`rc`] module, which uses `Rc`s for canning and has simplified aliases
//! (minimal generic arguments) for all the above types. For getting started
//! that `rc` module is probably the best place to start.
//!
//!
//!
//! ## Detailed Concept
//!
//! See the [Advanced Feature section of `Builder`].
//!
//! Also see [`Cache`], [`Builder`], [`blueprint`], [`canning`]
//!
//!
//![`Builder`]: trait.Builder.html
//![`Artifact`]: trait.Builder.html#associatedtype.Artifact
//![`build`]: trait.Builder.html#tymethod.build
//![`SimpleBuilder`]: rc/trait.SimpleBuilder.html
//![`rc`]: rc/index.html
//![`canning`]: canning/index.html
//![`blueprint`]: blueprint/index.html
//![`Blueprint`]: blueprint/struct.Blueprint.html
//![`Resolver`]: cache/struct.Resolver.html
//![`Cache`]: cache/struct.Cache.html
//![Advanced Feature section of `Builder`]: trait.Builder.html#advanced-features
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

#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "diagnostics")))]
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
/// This trait is use in contexts where [`Never`] appears. For instance this
/// trait is implemented on `Result<T,Never>` to unpack `T`, which is its only
/// value as `Never` is uninhabited i.e. can never exist.
///
/// One can think about unpacking as a compile-time guaranteed non-panicking
/// alternative to unwrapping. Therefore, if unpacking is available it should
/// be preferred over unwrapping.
///
/// # Example
///
/// ```
/// use daab::Never;
/// use daab::Unpacking;
///
/// let res: Result<u32, Never> = Ok(42);
/// // The error Never can never exist, thus we can unpack the result directly
/// // into the u32, compile-time guaranteed panic-free.
/// assert_eq!(42, res.unpack())
/// ```
///
/// [`Never`]: enum.Never.html
///
pub trait Unpacking {
	/// The type to be unpacked.
    type Inner;

	/// Unpacking into its inner value.
	///
	/// This function is guaranteed to never fail nor panic.
	///
    fn unpack(self) -> Self::Inner;
}
impl<T> Unpacking for Result<T,Never> {
    type Inner = T;

    fn unpack(self) -> T {
        unpack(self)
    }
}



/// Represents a Builder for an Artifact.
///
/// The `Builder` is the central trait of this crate. It defines the
/// _Builders_ (the structs implementing this trait) which are referred to
/// throughout this crate, as well as the _Artifacts_, which are the values
/// build by a Builder, and defined via the [`Artifact`] associate type.
///
/// To be usable within this crate, a Builder has to be wrapped in a
/// [`Blueprint`]. Then it can be used to with a [`Cache`] to build and get
/// its Artifact.
///
/// When `Blueprint` (containing a `Builder`) is resolved at a `Cache`, the
/// `Cache` will call the [`build`] method of that `Builder` as needed (i.e.
/// whenever there is no cached Artifact available),
/// providing it with a [`Resolver`], which allows to resolve its depending
/// Builders to their Artifacts.
///
/// An important concept is that a Builder may depend on other Builders (i.e.
/// it may use their Artifacts to construct its own Artifact). Thus constituting
/// existential dependencies between Artifacts.
/// The depending Builders are supposed to be stored in the `Builder` struct
/// which is then accessible from the `build` method to resolve them.
///
///
///
/// # Advanced Features
///
/// Unlike various `SimpleBuilder`s this `Builder` offers additionally more
/// advanced features than described above. These are explained in the
/// following.
///
///
/// ## Dynamic State
///
/// Each `Builder` may define a dynamic state, default is the unit type `()`.
/// That is a value that will be stored in a `Box` in the `Cache`, which will
/// be accessible even mutably for anyone from the `Cache`, as opposed to the
/// `Builder` itself, which will become inaccessible once wrapped in a
/// `Blueprint`.
///
/// If a `Builder` is encountered by a `Cache` for the first time, the `Cache`
/// will use the `Builder`'s `init_dyn_state` method to initialize the stored
/// dynamic state. It can then be accessed by the `Builder` itself from its
/// `build` method thought [`Resolver::my_state`] of the provided `Resolver`.
///
/// The dynamic state might be used for various purposes, the simples is as a
/// kind of variable configuration of the respective `Builder`. Notice that the
/// Artifact conceptional depends on the dynamic state, thus altering the
/// dynamic state (i.e. if access thought [`Cache::dyn_state_mut`])
/// will invalidate the Artifact of the respective `Builder`.
///
/// Another use-case of the dynamic state is to keep some state between builds.
/// An extreme example of this is the [`RedeemingBuilder`], which will replay
/// entire Artifacts of its inner Builder, when it fails to produce a new one.
///
///
/// ## Failure
///
/// `Builder`s are generally allowed to fail. Thus returning a `Result`
/// with the defined [`Err`] type,
/// which can be returned by the `build` method.
///
/// The infallible `SimpleBuilder`s use the [`Never`]-type (a stable variation
/// of the yet unstable `!`, the official `never`-type) as `Err`, because that
/// `Never` type allows simple [unpacking] of the `Result`s returned by the
/// `Cache`.
/// Thus if a Builder can always produce an Artifact, its `Err` type should be
/// that [`Never`] type.
///
///
///
/// [`Artifact`]: trait.Builder.html#associatedtype.Artifact
/// [`Blueprint`]: blueprint/struct.Blueprint.html
/// [`Cache`]: cache/struct.Cache.html
/// [`build`]: trait.Builder.html#tymethod.build
/// [`Resolver`]: cache/struct.Resolver.html
/// [`Resolver::my_state`]: cache/struct.Resolver.html#method.my_state
/// [`Cache::dyn_state_mut`]: cache/struct.Cache.html#method.dyn_state_mut
/// [`RedeemingBuilder`]: utils/struct.RedeemingBuilder.html
/// [`Never`]: enum.Never.html
/// [`Err`]: trait.Builder.html#associatedtype.Err
/// [unpacking]: trait.Unpacking.html
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

	/// Return an initial dynamic state for this builder.
	///
	/// When a builder is first seen by a `Cache` the cache will use this method
	/// to obtain an initial value for the dynamic state of this builder.
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
pub struct BuilderId(usize);

impl BuilderId {
	fn new(ptr: *const dyn Any) -> Self {
		BuilderId(ptr as *const () as usize)
	}

	fn as_ptr(&self) -> *const () {
		self.0 as *const ()
	}
}

impl fmt::Pointer for BuilderId {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		fmt::Pointer::fmt(&self.as_ptr(), fmt)
	}
}



// -----------

#[cfg(test)]
mod test;

#[cfg(test)]
mod multi_level_test;





