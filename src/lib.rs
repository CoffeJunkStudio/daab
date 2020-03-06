
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
//!     fn build(&self, _resolver: &mut rc::ArtifactResolverRc) -> Self::Artifact {
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
//!     builder_leaf: rc::ArtifactPromiseRc<BuilderLeaf>, // Dependency builder
//!     // ...
//! }
//! impl BuilderNode {
//!     pub fn new(builder_leaf: rc::ArtifactPromiseRc<BuilderLeaf>) -> Self {
//!         Self {
//!             builder_leaf,
//!             // ...
//!         }
//!     }
//! }
//! use std::any::Any;
//! impl Builder<Rc<dyn Any>, Rc<dyn Any>> for BuilderNode {
//!     type Artifact = Node;
//!     type DynState = u8;
//!     
//!     fn build(&self, resolver: &mut rc::ArtifactResolverRc<Self::DynState>) -> Rc<Self::Artifact> {
//!         // Resolve ArtifactPromise to its artifact
//!         let leaf = resolver.resolve(&self.builder_leaf);
//!         
//!         Rc::new(Node {
//!             leaf,
//!             value: resolver.get_my_user_data().copied().unwrap_or(42),
//!             // ...
//!         })
//!     }
//! }
//! 
//! // The cache to storing already created artifacts
//! let mut cache = rc::ArtifactCacheRc::new();
//!
//! // Constructing builders
//! let leaf_builder = ArtifactPromise::new(BuilderLeaf::new());
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
//! cache.invalidate(node_builder_1.clone());
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


use std::rc::Rc;
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

pub mod canning;

use canning::CanBase;
use canning::Can;
use canning::CanWithSize;

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
		where ArtCan: Can<Self::Artifact> {
	
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;
	
	// TODO: docs
	type DynState : Debug + 'static;
	
	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, cache: &mut ArtifactResolver<ArtCan, BCan, Self::DynState>) -> ArtCan::Bin;
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
pub struct ArtifactPromise<B, BCan: Can<B>> {
	builder: BCan::Bin,
	id: BuilderId,
}

impl<B, BCan: Can<B>> ArtifactPromise<B, BCan> {
	/// Crates a new promise for the given builder.
	///
	pub fn new(builder: B) -> Self
			where
				BCan: CanWithSize<B>, {
		
		let bin = BCan::into_bin(builder);
		let id = BuilderId(BCan::bin_as_ptr(&bin));
		
		ArtifactPromise {
			builder: bin,
			id,
		}
	}
	
	//pub into_can(self) -> ArtifactPromise
}

impl<B, BCan: Can<B>> Clone for ArtifactPromise<B, BCan> where BCan::Bin: Clone {
	fn clone(&self) -> Self {
		
		ArtifactPromise {
			builder: self.builder.clone(),
			id: self.id,
		}
	}
}

impl<B, BCan: Can<B>> Borrow<BuilderId> for ArtifactPromise<B, BCan> {
	fn borrow(&self) -> &BuilderId {
		&self.id
	}
}

impl<B, BCan: Can<B>> Hash for ArtifactPromise<B, BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

impl<B, BCan: Can<B>> PartialEq for ArtifactPromise<B, BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id.eq(&other.id)
	}
}

impl<B, BCan: Can<B>> Eq for ArtifactPromise<B, BCan> {
}

impl<B, BCan: Can<B>> fmt::Pointer for ArtifactPromise<B, BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", BCan::bin_as_ptr(&self.builder))
	}
}

impl<B, BCan: Can<B>> fmt::Debug for ArtifactPromise<B, BCan> where BCan::Bin: fmt::Debug {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "ArtifactPromise {{builder: {:?}, id: {:p}}}", self.builder, self.id)
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
pub struct ArtifactResolver<'a, ArtCan, BCan, T = ()> {
	user: &'a BuilderEntry<BCan>,
	cache: &'a mut ArtifactCache<ArtCan, BCan>,
	#[cfg(feature = "diagnostics")]
	diag_builder: &'a BuilderHandle<BCan>,
	_b: PhantomData<T>,
}

impl<'a, ArtCan: Debug, BCan: Clone + Debug, T: 'static> ArtifactResolver<'a, ArtCan, BCan, T> {
	/// Resolves the given `ArtifactPromise` into its artifact either by
	/// looking up the cached value in the associated `ArtifactCache` or by
	/// building it.
	///
	pub fn resolve<B: Builder<ArtCan, BCan> + 'static>(
		&mut self,
		promise: &ArtifactPromise<B, BCan>
	) -> ArtCan::Bin
			where
				ArtCan: Can<B::Artifact>,
				BCan: Can<B>,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B> {
		
		cfg_if! {
			if #[cfg(feature = "diagnostics")] {
				self.cache.do_resolve(self.user, self.diag_builder, promise)
			} else {
				self.cache.do_resolve(self.user, promise)
			}
		}
	}
	
	
	// TODO: consider whether mutable access is actually a good option
	// TODO: consider may be to even allow invalidation
	pub fn my_user_data(&mut self) -> &mut T {
		self.cache.get_dyn_state_cast(self.user.borrow()).unwrap()
	}
	
	pub fn get_my_user_data(&mut self) -> Option<&mut T> {
		self.cache.get_dyn_state_cast(self.user.borrow())
	}
	// TODO: docs
	pub fn get_user_data<B: Builder<ArtCan, BCan> + 'static>(
		&mut self,
		promise: &ArtifactPromise<B, BCan>
	) -> Option<&mut B::DynState> where BCan: Can<B>, ArtCan: Can<B::Artifact> {
		
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
struct BuilderId(*const dyn Any);

/*
impl<BCan, B: Builder<BCan> + 'static> From<&Rc<B>> for BuilderId {
	fn from(rc: &Rc<B>) -> Self {
		BuilderId(rc.as_ref() as &dyn Any as *const dyn Any)
	}
}
*/

impl fmt::Pointer for BuilderId {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		fmt::Pointer::fmt(&self.0, fmt)
	}
}


/// Auxiliary struct fro the `ArtifactCache` containing an untyped (aka
/// `dyn Any`) ArtifactPromise.
///
#[derive(Clone, Debug)]
pub struct BuilderEntry<BCan> {
	builder: BCan,
	id: BuilderId,
}

impl<BCan> BuilderEntry<BCan> {
	fn new<B: 'static>(value: ArtifactPromise<B, BCan>) -> Self
			where BCan: Can<B> {
		
		BuilderEntry {
			builder: BCan::from_bin(value.builder),
			id: value.id,
		}
	}
	
	/*
	fn downcast<ArtCan, B: Builder<ArtCan, BCan> + 'static>(self) -> ArtifactPromise<BCan::Bin>
			where BCan: Can<B>, ArtCan: Can<B::Artifact> {
		
		self.builder.downcast_can_ref()
	}
	*/
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
		writeln!(f, "{:p}", self.builder.as_ptr())
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
pub struct ArtifactCache<ArtCan, BCan, #[cfg(feature = "diagnostics")] T: ?Sized = dyn Doctor<ArtCan, BCan>> {
	/// Maps Builder-Capsules to their Artifact value
	cache: HashMap<BuilderEntry<BCan>, ArtCan>,
	
	/// Maps Builder-Capsules to their DynState value
	dyn_state: HashMap<BuilderId, Box<dyn Any>>,
	
	/// Tracks the direct promise dependants of each promise
	dependants: HashMap<BuilderId, HashSet<BuilderId>>,
	
	/// The doctor for error diagnostics.
	#[cfg(feature = "diagnostics")]
	doctor: T,
}

cfg_if! {
	if #[cfg(feature = "diagnostics")] {
		impl<ArtCan, BCan> Default for ArtifactCache<ArtCan, BCan, DefDoctor> {
			fn default() -> Self {
				ArtifactCache::new()
			}
		}
		
		impl<ArtCan: Debug, BCan: Debug, T: Debug> Debug for ArtifactCache<ArtCan, BCan, T> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "ArtifactCache {{ cache: {:?}, dependants: {:?}, doctor: {:?} }}",
					self.cache, self.dependants, self.doctor)
			}
		}

		impl<ArtCan, BCan> ArtifactCache<ArtCan, BCan, DefDoctor> {
			/// Creates a new empty cache with a dummy doctor.
			///
			pub fn new() -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependants: HashMap::new(),
					
					doctor: DefDoctor::default(),
				}
			}
		}

		impl<ArtCan, BCan, T: Doctor<ArtCan, BCan> + 'static> ArtifactCache<ArtCan, BCan, T> {
	
			/// Creates new empty cache with given doctor for inspection.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn new_with_doctor(doctor: T) -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependants: HashMap::new(),
					
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

		impl<ArtCan, BCan, T: Doctor<ArtCan, BCan> + 'static> Deref for ArtifactCache<ArtCan, BCan, T> {
			type Target = ArtifactCache<ArtCan, BCan>;
		
			fn deref(&self) -> &Self::Target {
				self
			}
		}

		impl<ArtCan, BCan, T: Doctor<ArtCan, BCan> + 'static> DerefMut for ArtifactCache<ArtCan, BCan, T> {
			fn deref_mut(&mut self) -> &mut Self::Target {
				self
			}
		}
		
		
	} else {
		impl<ArtCan, BCan> Default for ArtifactCache<ArtCan, BCan> {
			fn default() -> Self {
				ArtifactCache::new()
			}
		}
		
		impl<ArtCan: Debug, BCan: Debug> Debug for ArtifactCache<ArtCan, BCan> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "ArtifactCache {{ cache: {:?}, dependants: {:?} }}",
					self.cache, self.dependants)
			}
		}

		impl<ArtCan, BCan> ArtifactCache<ArtCan, BCan> {
			/// Creates a new empty cache.
			///
			pub fn new() -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependants: HashMap::new(),
				}
			}
		}
	}
}

/// Auxiliarry function to casts an `Option` of `Box` of `Any` to `T`.
///
/// Must only be used with the correct `T`, or panics.
///
fn cast_dyn_state<T: 'static>(v: Option<Box<dyn Any>>)
		-> Option<Box<T>> {
	
	v.map(
		|b| {
			// Ensure value type
			b.downcast()
				.expect("Cached Builder DynState is of invalid type")
		}
	)
}

impl<ArtCan: Debug, BCan: Debug> ArtifactCache<ArtCan, BCan> {
	
	/// Resolves the artifact of `promise` and records dependency between `user`
	/// and `promise`.
	///
	fn do_resolve<B: Builder<ArtCan, BCan> + 'static>(&mut self,
			user: &BuilderEntry<BCan>,
			#[cfg(feature = "diagnostics")]
			diag_builder: &BuilderHandle<BCan>,
			promise: &ArtifactPromise<B, BCan>) -> ArtCan::Bin
		
			where
				ArtCan: Can<B::Artifact>,
				BCan: Can<B>,
				BCan::Bin: Clone,
				BCan::Bin: AsRef<B> {
		
		
		let deps = self.get_dependants(&promise);
		if !deps.contains(user.borrow()) {
			deps.insert(*user.borrow());
		}
		
		#[cfg(feature = "diagnostics")]
		self.doctor.resolve(diag_builder, &BuilderHandle::new(promise.clone()));
		
		self.get(promise)
	}
	
	/// Returns the vector of dependants of promise
	///
	fn get_dependants<B: Builder<ArtCan, BCan> + 'static>(&mut self, promise: &ArtifactPromise<B, BCan>) -> &mut HashSet<BuilderId>
			where ArtCan: Can<B::Artifact>, BCan: Can<B> {
		if !self.dependants.contains_key(promise.borrow()) {
			self.dependants.insert(*promise.borrow(), HashSet::new());
		}
		
		self.dependants.get_mut(promise.borrow()).unwrap()
	}
	
	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup<B: Builder<ArtCan, BCan> + 'static>(
		&self,
		builder: &ArtifactPromise<B, BCan>
	) -> Option<ArtCan::Bin>
		where
			ArtCan: Can<B::Artifact>,
			BCan: Can<B> {
		
		// Get the artifact from the hash map ensuring integrity
		self.cache.get(&builder.id).map(
			|ent| {
				// Ensure value type
				ent.downcast_can()
					.expect("Cached Builder Artifact is of invalid type")
			}
		)
	}
	
	/// Store given artifact for given builder.
	///
	fn insert(&mut self, builder: BuilderEntry<BCan>, artifact: ArtCan) -> &ArtCan {
		
		let id = builder.id;
		
		// Insert artifact
		self.cache.insert(
			builder,
			artifact,
		);
		
		let c = self.cache.get(&id).unwrap();
		
		c
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
	pub fn get<B: Builder<ArtCan, BCan> + 'static>(
		&mut self,
		promise: &ArtifactPromise<B, BCan>
	) -> ArtCan::Bin
		where
			ArtCan: Can<B::Artifact>,
			BCan: Can<B>,
			BCan::Bin: Clone,
			BCan::Bin: AsRef<B> {
		
		if self.lookup(promise).is_some() {
			// No if-let because of borrow-checker error
			self.lookup(promise).unwrap()
			
		} else {
			let ent = BuilderEntry::new(promise.clone());
			
			#[cfg(feature = "diagnostics")]
			let diag_builder = BuilderHandle::new(promise.clone());
			
			let art_bin = promise.builder.as_ref().build(
				&mut ArtifactResolver {
					user: &ent,
					cache: self,
					#[cfg(feature = "diagnostics")]
					diag_builder: &diag_builder,
					_b: PhantomData,
				},
			);
			
			let art_can = ArtCan::from_bin(art_bin);
			
			
			#[cfg(feature = "diagnostics")]
			let value = art_can.downcast_can().unwrap();
			#[cfg(feature = "diagnostics")]
			self.doctor.build(&diag_builder, &ArtifactHandle::new(value));
			
			self.insert(ent,  art_can).downcast_can().unwrap()
		}
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
	
	// TODO: docs
	pub fn get_dyn_state<B: Builder<ArtCan, BCan> + 'static>(
		&mut self, promise: &ArtifactPromise<B, BCan>
	) -> Option<&mut B::DynState>
		where BCan: Can<B>, ArtCan: Can<B::Artifact> {
		
		
		self.get_dyn_state_cast(&promise.id)
	}
	
	// TODO: docs
	pub fn set_dyn_state<B: Builder<ArtCan, BCan> + 'static>(
		&mut self,
		promise: &ArtifactPromise<B, BCan>,
		user_data: B::DynState
	) -> Option<Box<B::DynState>>
		where BCan: Can<B>, ArtCan: Can<B::Artifact> {
		
		cast_dyn_state(
			self.dyn_state.insert(promise.id, Box::new(user_data))
		)
	}
	
	// TODO: add convenience function such as:
	// pub fn set_user_data_and_invalidate_on_change(...)
	
	// TODO: docs
	pub fn remove_dyn_state<B: Builder<ArtCan, BCan> + 'static>(&mut self, promise: &ArtifactPromise<B, BCan>)
			-> Option<Box<B::DynState>>
			where BCan: Can<B>, ArtCan: Can<B::Artifact> {
		
		cast_dyn_state(
			self.dyn_state.remove(&promise.id)
		)
	}
	
	// TODO: docs
	pub fn clear_dyn_state(&mut self) {
		
		self.dyn_state.clear();
	}
	
	// TODO: consider whether dynamic state shall survive invalidation or not
	
	/// Clears the entire cache including all kept promise and artifact `Rc`s.
	///
	pub fn clear(&mut self) {
		self.cache.clear();
		self.dyn_state.clear();
		self.dependants.clear();
		
		#[cfg(feature = "diagnostics")]
		self.doctor.clear();
	}
	
	/// Auxiliary invalidation function using an untyped (aka `dyn Any`) `BuilderId`.
	///
	fn invalidate_any(&mut self, builder: BuilderId) {
		if let Some(set) = self.dependants.remove(&builder) {
			for dep in set {
				self.invalidate_any(dep);
			}
		}
		
		self.cache.remove(&builder);
	}
	
	/// Removes the given promise with its cached artifact from the cache and
	/// all depending artifacts (with their promises).
	///
	/// Depending artifacts are all artifacts which used the former during
	/// its building. The dependencies are automatically tracked via the
	/// `ArtifactResolver`.
	///
	pub fn invalidate<B: Builder<ArtCan, BCan> + 'static>(&mut self, promise: ArtifactPromise<B, BCan>)
			where
				BCan: Can<B>,
				ArtCan: Can<B::Artifact> {
		
		self.invalidate_any(promise.id);
		
		#[cfg(feature = "diagnostics")]
		self.doctor.invalidate(&BuilderHandle::new(promise));
	}
}





// -----------

#[cfg(test)]
mod test;

#[cfg(test)]
mod multi_level_test;





