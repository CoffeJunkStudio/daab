//!
//! Artifact caching structures
//!
//! This module contains the [`Cache`] and its pendant for Builders the
//! [`Resolver`]. The cache is the entrance point to retrieve the Artifacts from
//! Builders. It ensures the correct building of Artifacts if not known yet
//! and keeping them to allow looking them up repeatedly, without rebuilding
//! them.
//!
//! Notice: both types here are generic over their Artifact-Can (`ArtCan`) and
//! Builder-Can (`BCan`), for details about this advanced concept refer to
//! [crate description], for details about Cans see the [`canning`] module,
//! for simplified aliases (those with preset `ArtCan` & `BCan`) see any of the
//! alias modules: [`rc`], [`arc`], [`boxed`].
//!
//![`Cache`]: struct.Cache.html
//![`Resolver`]: struct.Resolver.html
//![crate description]: ../index.html#detailed-concept
//![`canning`]: ../canning/index.html
//![`rc`]: ../rc/index.html
//![`arc`]: ../arc/index.html
//![`boxed`]: ../boxed/index.html
//!


use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;

use cfg_if::cfg_if;

#[cfg(feature = "mut_box")]
use crate::canning::CanRefMut;

use crate::CanStrong;
use crate::CanSized;
use crate::CanRef;
use crate::Can;

use crate::Promise;

use crate::Builder;

mod internal;

use internal::BuilderEntry;
use internal::RawCache;



/// Structure for building, caching and dependency tracking of artifacts.
///
/// The `Cache` is the central structure of this crate. It helps to
/// avoid dependency duplication when multiple Builders depend on the same
/// Artifact.
///
/// Since all `Builder`s in the context of this crate are supposed to be wrapped
/// within `Blueprint`, the `Cache` is the only way of acquiring
/// an artifact in the first place.
///
/// Notice In the debugging version (when the **`diagnostics`** feature is active),
/// this struct contains a debugger `Doctor`, which
/// allows run-time inspection of various events.
/// In order to store it, the **`diagnostics`** `Cache` is generic to
/// some `Doctor`.
/// The `new` method then returns a `Cache<NoopDoctor>`
/// and `new_with_doctor` returns some `Cache<T>`.
///
/// Only an `Cache<T>` with `T: Sized` can be store in variables.
/// However, since most of the code does not care about the concrete
/// Doctor the default generic is `dyn Doctor`, on which all other methods are
/// defined.
/// An `Cache<dyn Doctor>` can not be stored, but it can be passed
/// on by reference (e.g. as `&mut Cache`). This prevents the use of
/// additional generics in **`diagnostics`** mode, and allows to easier achieve
/// compatibility between **`diagnostics`** and non-**`diagnostics`** mode.
/// To ease conversion between `Cache<T>` and
/// `Cache<dyn Doctor>` (aka `Cache`), all creatable
/// `Cache<T>`s (with `T: Sized`) implement `DerefMut`
/// to `Cache<dyn Doctor>`.
///
///
///
/// ## Artifact Accessors
///
/// The `Cache` provides the following set of accessors:
///
/// Basically there are the `get*` and the `lookup*` methods. The former will
/// create the artifact if it does not exist, but may return a build-error.
/// The latter directly returns `Some` Artifact or `None` if it is not in cache.
///
/// Each `get*` and `lookup*`, come in 4 different variants depending offering
/// different access semantics to the artifact.
/// - `Bin<T>` variant (`get` & `lookup`) returns a clone of the respective Bin
///    of the Artifact that is e.g. an `Rc<T>` when using `Rc<dyn Any>` as Can.
/// - `&T` variant (`get_ref` & `lookup_ref`) returns a reference to artifact
///   within this `Cache`.
/// - `&mut T` variant (`get_mut` & `lookup_mut`) returns a mutable reference
///   to artifact within this `Cache`.
/// - `T` variant (`lookup_cloned` & `get_cloned`) returns a clone of the
///   Artifact itself.
///
/// |           |`Bin<T>`| `&T`       | `&mut T`   | `T`           |
/// |-----------|--------|------------|------------|---------------|
/// |`Option`   |`lookup`|`lookup_ref`|`lookup_mut`|`lookup_cloned`|
/// |`Result`   |`get`   |`get_ref`   |`get_mut`   |`get_cloned`   |
/// |usable with|`rc`,`arc`|`rc`,`arc`,`boxed`|`boxed`|`rc`,`arc`,`boxed`|
///
/// _`Bin<T>` means `<ArtCan as Can<T>>::Bin` \
/// e.g. `Rc<T>` for types in `rc` module_
///
///
///
/// ## Caching Duration
///
/// Since this struct is a cache, it might keep the artifacts indefinitely,
/// which could cause memory-leak-like issues. In the following the precise
/// duration for storing Artifacts, dynamic states and Builders are described.
///
/// The `dyn_state` and `dyn_state_mut` will create a dynamic state for the
/// respective Builder if it does not exist yet in the `Cache`, thus
/// allocating memory.
///
/// Similarly, `get`, `get_ref`, `get_mut`, and `get_cloned` will produce the
/// Artifact with all dependent Artifacts, it will also allocate the dynamic
/// state for all Builders of those Artifacts, if it does not exist yet.
///
/// On the other hand, `get_dyn_state`, `lookup`, `lookup_ref`, `lookup_mut`,
/// and `lookup_cloned` methods will never add anything to the `Cache`.
///
/// With [`invalidate`], a specific Artifact can be removed from the `Cache`,
/// which will invalidate (means remove) all Artifacts that depended on it.
/// But this will never remove any dynamic state.
///
/// Whenever there is dynamic state or Artifact of an Builder in the cache,
/// the respective Builder will be kept too in a "weak" Can, that is
/// `std::rc::Weak<dyn Any>` when using the `rc` module. This will however keep
/// the entire memory of the respective Builder allocated, even when all
/// "strong" Cans are out of scope.
///
/// In order to remove also the dynamic state and thus the Builder Can form the
/// Cache there exists the [`purge`] method. However, it will only purge the
/// given builder, and invalidate the depending Artifact.
///
/// If there are (multiple) Builders go out of scope, they will not be removed
/// form this `Cache`. In order to remove _unreachable_ Builders including their
/// Artifacts and dynamic state, there is the [`garbage_collection`] method,
/// which will go through all the cached "weak" Builders and purge all that
/// became unreachable.
///
/// For diagnostics there are the [`is_builder_known`] and
/// [`number_of_known_builders`] methods.
///
/// To clear all Artifacts there is the [`clear_artifacts`] method. And to
/// purge all Builders from the `Cache` there is the [`clear_all`] method.
///
/// [`invalidate`]: struct.Cache.html#method.invalidate
/// [`purge`]: struct.Cache.html#method.purge
/// [`garbage_collection`]: struct.Cache.html#method.garbage_collection
/// [`clear_artifacts`]: struct.Cache.html#method.clear_artifacts
/// [`clear_all`]: struct.Cache.html#method.clear_all
/// [`is_builder_known`]: struct.Cache.html#method.is_builder_known
/// [`number_of_known_builders`]: struct.Cache.html#method.number_of_known_builders
///
pub struct Cache<
	ArtCan,
	BCan,
	#[cfg(feature = "diagnostics")] Doc: ?Sized = dyn Doctor<ArtCan, BCan>
> where BCan: CanStrong {

	/// The inner cache
	#[cfg(feature = "diagnostics")]
	inner: RawCache<ArtCan, BCan, Doc>,
	#[cfg(not(feature = "diagnostics"))]
	inner: RawCache<ArtCan, BCan>,

}

/// The ownable and storable variant of the Cache.
///
/// This is a simple type-def to Cache, which guarantees independent of
/// whether the `diagnostics` feature is enabled or not that this type is
/// storable as owned value.
///
/// When ever a Cache needs to be stored such as in a struct, this type
/// alias should be preferred over using Cache directly.
///
#[cfg(feature = "diagnostics")]
pub type CacheOwned<ArtCan, BCan> =
	Cache<ArtCan, BCan, DefDoctor>;

/// The ownable and storable variant of the Cache.
///
/// This is a simple type-def to Cache, which guarantees independent of
/// whether the `diagnostics` feature is enabled or not that this type is
/// storable as owned value.
///
/// When ever a Cache needs to be stored such as in a struct, this type
/// alias should be preferred over using Cache directly.
///
#[cfg(not(feature = "diagnostics"))]
pub type CacheOwned<ArtCan, BCan> =
	Cache<ArtCan, BCan>;

impl<ArtCan, BCan> Default for CacheOwned<ArtCan, BCan>
	where BCan: CanStrong {

	fn default() -> Self {
		CacheOwned::new()
	}
}

impl<ArtCan, BCan> CacheOwned<ArtCan, BCan>
	where BCan: CanStrong {

	/// Creates a new empty cache with a dummy doctor.
	///
	pub fn new() -> Self {
		cfg_if! {
			if #[cfg(feature = "diagnostics")] {
				Self {
					inner: RawCache::new_with_doctor(Default::default())
				}
			} else {
				Self {
					inner: RawCache::new()
				}
			}
		}
	}
}

cfg_if! {
	if #[cfg(feature = "diagnostics")] {
		use std::ops::Deref;
		use std::ops::DerefMut;
		use crate::Doctor;
		use crate::DefDoctor;
		use crate::BuilderHandle;

		impl<ArtCan, BCan, Doc> Debug for Cache<ArtCan, BCan, Doc>
			where ArtCan: Debug, BCan: CanStrong + Debug, Doc: Debug {

			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				self.inner.fmt(f)
			}
		}

		impl<ArtCan, BCan, Doc> Cache<ArtCan, BCan, Doc>
			where BCan: CanStrong, Doc: Doctor<ArtCan, BCan> + 'static {

			/// Creates new empty cache with given doctor for inspection.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn new_with_doctor(doctor: Doc) -> Self {
				Self {
					inner: RawCache::new_with_doctor(doctor)
				}
			}

			/// Returns a reference of the inner doctor.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn doctor(&mut self) -> &mut Doc {
				&mut self.inner.doctor
			}

			/// Consumes the `Cache` and returns the inner doctor.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn into_doctor(self) -> Doc {
				self.inner.doctor
			}
		}

		impl<ArtCan, BCan, Doc> Deref for Cache<ArtCan, BCan, Doc>
			where BCan: CanStrong, Doc: Doctor<ArtCan, BCan> + 'static {

			type Target = Cache<ArtCan, BCan>;

			fn deref(&self) -> &Self::Target {
				self
			}
		}

		impl<ArtCan, BCan, Doc> DerefMut for Cache<ArtCan, BCan, Doc>
			where BCan: CanStrong, Doc: Doctor<ArtCan, BCan> + 'static {

			fn deref_mut(&mut self) -> &mut Self::Target {
				self
			}
		}


	} else {
		impl<ArtCan, BCan> Debug for Cache<ArtCan, BCan>
			where ArtCan: Debug, BCan: CanStrong + Debug {

			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				self.inner.fmt(f)
			}
		}
	}
}

impl<ArtCan: Debug, BCan: CanStrong + Debug> Cache<ArtCan, BCan> {

	/// Tests whether the artifact or dynamic state of the given builder is
	/// recorded in this cache.
	///
	/// If this function returns `true`, the artifact or dynamic state or both
	/// are kept in this `Cache` and additionally a "weak" Can to the Builder.
	/// That is `rc::Weak<dyn Any>` when using the `rc` module.
	/// If this function returns `false`, it is guaranteed that none of the
	/// above are kept in this `Cache`.
	///
	/// [`number_of_known_builders`] returns the amount of Builders for
	/// which this methods returns `true`.
	///
	/// [`number_of_known_builders`]: struct.Cache.html#method.number_of_known_builders
	///
	pub fn is_builder_known<AP: ?Sized>(
			&self,
			promise: &AP
		) -> bool
			where
				BCan: Can<AP::Builder>,
				AP: Promise<BCan = BCan> {

		self.inner.is_builder_known(promise)
	}

	/// Gets the stored Artifact in its Bin, if it exists.
	///
	/// Returns the Artifact in its Bin. That is an `Rc<B::Artifact>` when using
	/// the `rc` module. The Bin is useful to share an identical artifact
	/// or one that is not `Clone` when an owned value is required or lifetime
	/// errors occur using [`lookup_ref`].
	///
	/// This method will not attempt to build the Artifact if it does not exist
	/// already, instead `None` will be returned then.
	///
	/// For an overview of different accessor methods see [Artifact Accessors]
	/// section of `Cache`.
	///
	/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
	/// [`lookup_ref`]: struct.Cache.html#method.lookup_ref
	///
	pub fn lookup<AP, B: ?Sized>(
			&self,
			promise: &AP
		) -> Option<ArtCan::Bin>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.lookup(promise)
	}

	/// Gets the stored Artifact by reference, if it exists.
	///
	/// Returns the Artifact as reference into this `Cache`. The reference is
	/// useful to access artifact for short time, as it dose not incur any
	/// cloning overhead, thus it is the cheapest way to access an Artifact, and
	/// should be preferred wherever possible.
	///
	/// When an owned value is required instead or lifetime issues arise,
	/// [`lookup`] and [`lookup_cloned`] are alternatives, which return a clone
	/// of the Artifact Bin or of the Artifact itself, respectively.
	///
	/// Also notice, that using some special `ArtCan`s such as using the
	/// [`boxed`] module there also exists a [`lookup_mut`] for mutable access
	/// to the artifact stored within this `Cache`.
	///
	/// This method will not attempt to build the Artifact if it does not exist
	/// already, instead `None` will be returned then.
	///
	/// For an overview of different accessor methods see [Artifact Accessors]
	/// section of `Cache`.
	///
	/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
	/// [`lookup`]: struct.Cache.html#method.lookup
	/// [`boxed`]: ../boxed/index.html
	/// [`lookup_cloned`]: struct.Cache.html#method.lookup_cloned
	/// [`lookup_mut`]: struct.Cache.html#method.lookup_mut
	///
	pub fn lookup_ref<AP, B: ?Sized>(
			&self,
			promise: &AP
		) -> Option<&B::Artifact>
			where
				ArtCan: CanRef<B::Artifact>,
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.lookup_ref(promise)
	}


cfg_if! {
	if #[cfg(feature = "mut_box")] {

		/// Gets the stored Artifact by mutable reference, if it exists.
		///
		/// Returns the Artifact as mutable reference into this `Cache`.
		/// The mutable reference might be useful to access the Artifact in place
		/// to mutate it.
		///
		/// Note: If mutation is not required [`lookup_ref`] should be
		/// preferred. Also if mutation is conditional, first a shared reference
		/// should be acquired via [`lookup_ref`] to test the condition and only
		/// when necessary a mutable reference should be acquired.
		///
		/// **Currently, when using this method, all artifacts which depended on
		/// accessed one will be invalidate!**
		///
		/// This method will not attempt to build the Artifact if it does not exist
		/// already, instead `None` will be returned then.
		///
		/// For an overview of different accessor methods see [Artifact Accessors]
		/// section of `Cache`.
		///
		///
		///
		/// # Unstable
		///
		/// This function should be only used with care, because currently, this
		/// function will invalidate all depending artifacts, **but this is subject
		/// to change**.
		///
		/// Therefore, **this method must be considered unstable!** The semantic of
		/// this function might change in a breaking way within a non-breaking
		/// version update!
		///
		///
		/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
		/// [`lookup_ref`]: struct.Cache.html#method.lookup_ref
		///
		#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "mut_box")))]
		pub fn lookup_mut<AP, B: ?Sized>(
				&mut self,
				promise: &AP
			) -> Option<&mut B::Artifact>
				where
					ArtCan: CanRefMut<B::Artifact>,
					B: Builder<ArtCan, BCan>,
					BCan: Can<AP::Builder>,
					AP: Promise<Builder = B, BCan = BCan>  {

			self.inner.lookup_mut(promise)
		}
	}
}

	/// Gets a clone of the stored Artifact, if it exists.
	///
	/// Returns a clone of the Artifact. The clone is useful when cloning the
	/// Artifact itself is viable and an owned value is required or lifetime
	/// errors occur using [`lookup_ref`].
	///
	/// This method will not attempt to build the Artifact if it does not exist
	/// already, instead `None` will be returned then.
	///
	/// For an overview of different accessor methods see [Artifact Accessors]
	/// section of `Cache`.
	///
	/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
	/// [`lookup_ref`]: struct.Cache.html#method.lookup_ref
	///
	pub fn lookup_cloned<AP, B: ?Sized>(
			&self,
			promise: &AP
		) -> Option<B::Artifact>
			where
				ArtCan: CanRef<B::Artifact>,
				B: Builder<ArtCan, BCan>,
				B::Artifact: Clone,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.lookup_cloned(promise)
	}

	/// Gets the Artifact in its Bin.
	///
	/// Returns the Artifact in its Bin. That is an `Rc<B::Artifact>` when using
	/// the `rc` module. The Bin is useful to share an identical artifact
	/// or one that is not `Clone` when an owned value is required or lifetime
	/// errors occur using [`get_ref`].
	///
	/// This method will try to build the Artifact if it is not stored in the
	/// `Cache`. The building using the Builder's `build` method could fail,
	/// thus a `Result` is returned. An `Err` will be returned only, if the
	/// Artifact was not cached and the Builder returned an `Err`.
	///
	/// For an overview of different accessor methods see [Artifact Accessors]
	/// section of `Cache`.
	///
	/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
	/// [`get_ref`]: struct.Cache.html#method.get_ref
	///
	pub fn get<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<ArtCan::Bin, B::Err>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.get(promise)
	}

	/// Gets the Artifact by reference.
	///
	/// Returns the Artifact as reference into this `Cache`. The reference is
	/// useful to access artifact for short time, as it dose not incur any
	/// cloning overhead, thus it is the cheapest way to access an Artifact, and
	/// should be preferred wherever possible.
	///
	/// When an owned value is required instead or lifetime issues arise,
	/// [`get`] and [`get_cloned`] are alternatives, which return a clone
	/// of the Artifact Bin or of the Artifact itself, respectively.
	///
	/// Also notice, that using some special `ArtCan`s such as using the
	/// [`boxed`] module there also exists a [`get_mut`] for mutable access
	/// to the artifact stored within this `Cache`.
	///
	/// This method will try to build the Artifact if it is not stored in the
	/// `Cache`. The building using the Builder's `build` method could fail,
	///  thus a `Result` is returned. An `Err` will be returned only, if the
	/// Artifact was not cached and the Builder returned an `Err`.
	///
	/// For an overview of different accessor methods see [Artifact Accessors]
	/// section of `Cache`.
	///
	/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
	/// [`get`]: struct.Cache.html#method.get
	/// [`get_cloned`]: struct.Cache.html#method.get_cloned
	/// [`boxed`]: ../boxed/index.html
	/// [`get_mut`]: struct.Cache.html#method.get_mut
	///
	pub fn get_ref<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<&B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.get_ref(promise)
	}


cfg_if! {
	if #[cfg(feature = "mut_box")] {
		/// Gets the Artifact by mutable reference.
		///
		/// Returns the Artifact as mutable reference into this `Cache`.
		/// The mutable reference might be useful to access the Artifact in place
		/// to mutate it.
		///
		/// Note: If mutation is not required [`get_ref`] should be
		/// preferred. Also if mutation is conditional, first a shared reference
		/// should be acquired via [`get_ref`] to test the condition and only
		/// when necessary a mutable reference should be acquired.
		///
		/// **Currently, when using this method, all artifacts which depended on
		/// accessed one will be invalidate!**
		///
		/// This method will try to build the Artifact if it is not stored in the
		/// `Cache`. The building using the Builder's `build` method could fail,
		///  thus a `Result` is returned. An `Err` will be returned only, if the
		/// Artifact was not cached and the Builder returned an `Err`.
		///
		/// For an overview of different accessor methods see [Artifact Accessors]
		/// section of `Cache`.
		///
		///
		///
		/// # Unstable
		///
		/// This function should be only used with care, because currently, this
		/// function will invalidate all depending artifacts, **but this is subject
		/// to change**.
		///
		/// Therefore, **this method must be considered unstable!** The semantic of
		/// this function might change in a breaking way within a non-breaking
		/// version update!
		///
		///
		/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
		/// [`get_ref`]: struct.Cache.html#method.get_ref
		///
		#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "mut_box")))]
		pub fn get_mut<AP, B: ?Sized>(
				&mut self,
				promise: &AP
			) -> Result<&mut B::Artifact, B::Err>
				where
					ArtCan: CanRefMut<B::Artifact>,
					B: Builder<ArtCan, BCan>,
					BCan: Can<AP::Builder>,
					AP: Promise<Builder = B, BCan = BCan>  {

			self.inner.get_mut(promise)
		}
	}
}

	/// Get a clone of the Artifact.
	///
	/// Returns a clone of the Artifact. The clone is useful when cloning the
	/// Artifact itself is viable and an owned value is required or lifetime
	/// errors occur using [`get_ref`].
	///
	/// This method will try to build the Artifact if it is not stored in the
	/// `Cache`. The building using the Builder's `build` method could fail,
	/// thus a `Result` is returned. An `Err` will be returned only, if the
	/// Artifact was not cached and the Builder returned an `Err`.
	///
	/// For an overview of different accessor methods see [Artifact Accessors]
	/// section of `Cache`.
	///
	/// [Artifact Accessors]: struct.Cache.html#artifact-accessors
	/// [`get_ref`]: struct.Cache.html#method.get_ref
	///
	pub fn get_cloned<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				B: Builder<ArtCan, BCan>,
				B::Artifact: Clone,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.get_cloned(promise)
	}

	/// Gets the dynamic state of the given builder, if any.
	///
	/// To initialize the dynamic state when it does not exist, use the
	/// [`dyn_state`] method instead.
	///
	/// [`dyn_state`]: struct.Cache.html#method.dyn_state
	///
	pub fn get_dyn_state<AP, B: ?Sized>(
			&self, promise: &AP
		) -> Option<&B::DynState>
			where
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.get_dyn_state(promise)
	}

	/// Gets the dynamic state of the given Builder.
	///
	/// This method will initialize the dynamic state if
	/// it didn't exist yet in this `Cache`.
	///
	/// Alternatively, [`dyn_state_mut`] can be used to acquire mutable access
	/// to the dynamic state in order to "reconfigure" the Builder. Or
	/// [`get_dyn_state`] to only access the dynamic state if it already exists.
	///
	/// [`get_dyn_state`]: struct.Cache.html#method.get_dyn_state
	/// [`dyn_state_mut`]: struct.Cache.html#method.dyn_state_mut
	///
	pub fn dyn_state<AP, B: ?Sized>(
			&mut self, promise: &AP
		) -> &B::DynState
			where
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.dyn_state(promise)
	}

	/// Gets the mutable dynamic state of the given Builder.
	///
	/// This method will initialize the dynamic state if
	/// it didn't exist yet in this `Cache`.
	///
	/// As opposed to [`dyn_state`], this method will invalidate the Artifact of
	/// the given Builder, including all depending Artifacts.
	///
	/// [`dyn_state`]: struct.Cache.html#method.dyn_state
	///
	pub fn dyn_state_mut<AP, B: ?Sized>(
			&mut self, promise: &AP
		) -> &mut B::DynState
			where
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.dyn_state_mut(promise)
	}

	/// Deletes all cached Artifacts in this cache, but keeps dynamic states.
	///
	pub fn clear_artifacts(&mut self) {
		self.inner.clear_artifacts()
	}

	/// Clears the entire cache including all kept Builders, Artifacts and
	/// dynamic states.
	///
	pub fn clear_all(&mut self) {
		self.inner.clear_all()
	}

	/// Deletes the artifact and the dynamic state of the given builder.
	///
	/// This function has the effect that all references of the given builder
	/// held by this cache will be removed.
	///
	/// As a consequence, all depending Artifacts (if any) will be invalidated,
	/// but their dynamic states will be kept.
	///
	/// If you only want to remove the artifact you can use [`invalidate`]
	/// instead.
	///
	/// [`invalidate`]: struct.Cache.html#method.invalidate
	///
	pub fn purge<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.purge(promise)
	}

	/// Removes the Artifact of the given Builder from the `Cache` and
	/// all depending Artifacts, but keep their dynamic states.
	///
	/// Depending Artifacts are all Artifacts, which used the former during
	/// their building. The dependencies are automatically tracked via the
	/// [`Resolver`] provided to the [`build`] method of a Builder.
	///
	/// In order to remove also the dynamic state of the given builder use the
	/// [`purge`] method.
	///
	/// [`purge`]: struct.Cache.html#method.purge
	/// [`Resolver`]: struct.Resolver.html
	/// [`build`]: ../trait.Builder.html#tymethod.build
	///
	pub fn invalidate<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				B: Debug + 'static,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.inner.invalidate(promise)
	}

	/// Invalidates all builders and their dyn state which can not be builded
	/// any more, because there are no more references to them.
	///
	/// This function has the complexity of `O(n)` with `n` being the [number of
	/// known builders]. Thus this function is not light-weight, but should be
	/// called regularly at appropriate locations, i.e. where many builders
	/// go out of scope.
	///
	/// If builders and dynamic states are explicitly removed before going out
	/// of scope e.g. via [`purge`] or [`clean_all`], this method is not needed
	/// to be called in order to prevent memory leakage. Also notice, this
	/// method only takes efforts to reduce resource leakage, but can't give
	/// any guarantees, nor has any functional impact.
	///
	/// Only those Builders will be cleaned up, for which all Cans and Bins
	/// (e.g. `Rc`s) have been dropped. If Artifacts or dynamic states refers to
	/// Builders, those Builders might need additional GC cycles to be cleaned
	/// up, or in case of cyclic dependencies (e.g. between dynamic states and
	/// Artifacts) might never be cleaned by this GC.
	///
	/// [number of known builders]: struct.Cache.html#method.number_of_known_builders
	/// [`purge`]: struct.Cache.html#method.purge
	/// [`clean_all`]: struct.Cache.html#method.clean_all
	///
	pub fn garbage_collection(&mut self) {
		self.inner.garbage_collection()
	}

	/// Returns the number of currently kept artifact promises.
	///
	/// This method is offered as kind of debugging or analysis tool for
	/// keeping track of the number of active Builders.
	///
	/// When adding dynamic state or issuing the building of a promise may
	/// increase the returned number. Like wise the purging of Builders
	/// may decrement this count. Additionally, if there are
	/// no more usable references to a Builder, the [`garbage_collection`]
	/// method may reduce this number.
	///
	/// [`is_builder_known`] can test whether individual Builders are stored in
	/// this cache. `number_of_known_builders` returns exactly how many
	/// Builders exist for which `is_builder_known` returns `true`.
	///
	/// [`is_builder_known`]: struct.Cache.html#method.is_builder_known
	/// [`garbage_collection`]: struct.Cache.html#method.garbage_collection
	///
	pub fn number_of_known_builders(&self) -> usize {
		self.inner.number_of_known_builders()
	}
}





/// Resolves dependent Artifacts for Builders.
///
/// This struct is only available to the [`build`] method of Builders. It is
/// specific to that Builder, which will be referred to as the _owning
/// Builder_. The `Resolver` provides limited access to the [`Cache`] for which
/// the owning Builder builds its Artifact. This access is limited to the
/// dynamic state of the owning Builder and the Artifacts of other builders.
///
/// This concept of a specific `Resolver` serves the important purpose of
/// tracking dependencies between Artifacts. Thus all Artifacts which are
/// retrieved through a `Resolver` create a _dependency_ of the Artifact of
/// the owning Builder upon the resolved Artifact.
///
/// These tracked dependencies are used to correctly implement the Artifact
/// invalidation of the `Cache` through [`Cache::invalidate`].
///
/// [`build`]: ../trait.Builder.html#tymethod.build
/// [`Cache`]: struct.Cache.html
/// [`Cache::invalidate`]: struct.Cache.html#method.invalidate
///
pub struct Resolver<'a, ArtCan, BCan: CanStrong, DynState = ()> {
	user: &'a BuilderEntry<BCan>,
	cache: &'a mut RawCache<ArtCan, BCan>,
	#[cfg(feature = "diagnostics")]
	diag_builder: &'a BuilderHandle<BCan>,
	_b: PhantomData<DynState>,
}

impl<'a, ArtCan, BCan, DynState> Resolver<'a, ArtCan, BCan, DynState>
	where
		ArtCan: Debug,
		BCan: CanStrong,
		DynState: 'static, {

	/// Record a dependency upon the given promise.
	///
	fn track_dependency<AP>(
			&mut self,
			promise: &AP
		)
			where
				BCan: Can<AP::Builder>,
				AP: Promise<BCan = BCan> {

		cfg_if! {
			if #[cfg(feature = "diagnostics")] {
				self.cache.track_dependency(
					self.user, self.diag_builder, promise)
			} else {
				self.cache.track_dependency(
					self.user, promise)
			}
		}
	}


	/// Resolves an Artifact to its Bin.
	///
	/// Returns the Artifact in its Bin. That is an `Rc<B::Artifact>` when using
	/// the `rc` module. The Bin is useful to share an identical artifact
	/// or one that is not `Clone` when an owned value is required or lifetime
	/// errors occur using [`resolve_ref`].
	///
	/// This method will try to build the Artifact if it is not stored in the
	/// corresponding `Cache`. The building using that Builder's `build` method
	/// could fail, thus a `Result` is returned. An `Err` will be returned
	/// only, if the Artifact was not cached and the Builder returned an `Err`.
	///
	/// Also see the corresponding [`get`] method of `Cache`.
	///
	/// [`resolve_ref`]: struct.Resolver.html#method.resolve_ref
	/// [`get`]: struct.Cache.html#method.get
	///
	pub fn resolve<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<ArtCan::Bin, B::Err>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan> {

		self.track_dependency(promise);
		self.cache.get(promise)
	}

	/// Resolves an Artifact by reference.
	///
	/// Returns the Artifact as reference into the corresponding `Cache`. The
	/// reference is useful to access artifact for short time, as it dose not
	/// incur any cloning overhead, thus it is the cheapest way to access an
	/// Artifact, and should be preferred wherever possible.
	///
	/// When an owned value is required instead or lifetime issues arise,
	/// [`resolve`] and [`resolve_cloned`] are alternatives, which return a
	/// clone of the Artifact Bin or of the Artifact itself, respectively.
	///
	/// This method will try to build the Artifact if it is not stored in the
	/// `Cache`. The building using the Builder's `build` method could fail,
	///  thus a `Result` is returned. An `Err` will be returned only, if the
	/// Artifact was not cached and the Builder returned an `Err`.
	///
	/// Also see the corresponding [`get_ref`] method of `Cache`.
	///
	/// [`resolve`]: struct.Resolver.html#method.resolve
	/// [`resolve_cloned`]: struct.Resolver.html#method.resolve_cloned
	/// [`get_ref`]: struct.Cache.html#method.get_ref
	///
	pub fn resolve_ref<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<&B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				B: Builder<ArtCan, BCan>,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.track_dependency(promise);
		self.cache.get_ref(promise)
	}

	/// Resolves an Artifact into a clone of it.
	///
	/// Returns a clone of the Artifact. The clone is useful when cloning the
	/// Artifact itself is viable and an owned value is required or lifetime
	/// errors occur using [`resolve_ref`].
	///
	/// This method will try to build the Artifact if it is not stored in the
	/// `Cache`. The building using the Builder's `build` method could fail,
	/// thus a `Result` is returned. An `Err` will be returned only, if the
	/// Artifact was not cached and the Builder returned an `Err`.
	///
	/// Also see the corresponding [`get_cloned`] method of `Cache`.
	///
	/// [`resolve_ref`]: struct.Resolver.html#method.resolve_ref
	/// [`get_cloned`]: struct.Cache.html#method.get_cloned
	///
	pub fn resolve_cloned<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				B: Builder<ArtCan, BCan>,
				B::Artifact: Clone,
				BCan: Can<AP::Builder>,
				AP: Promise<Builder = B, BCan = BCan>  {

		self.track_dependency(promise);
		self.cache.get_cloned(promise)
	}

	/// Returns the dynamic state of the owning Builder.
	///
	/// Notice, when an Artifact needs to be builded, the dynamic state of the
	/// respective Builder will be initialized preventively, thus this method
	/// wan always return a dynamic state without the need to create it. In
	/// other words when an Artifact is build, it will get an dynamic state,
	/// regardless wether this method or and other dynamic state accessor is
	/// ever called.
	///
	pub fn my_state(&mut self) -> &mut DynState {
		// The unwrap is safe here, because Cache ensures that a DynState exists
		// before we comme here.
		self.cache.dyn_state_cast_mut(self.user.id()).unwrap()
	}
}


