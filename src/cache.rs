


use std::borrow::Borrow;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;

use cfg_if::cfg_if;

use crate::CanStrong;
use crate::CanSized;
use crate::CanRef;
use crate::CanRefMut;

use crate::Promise;

use crate::Builder;

mod internal;

use internal::BuilderEntry;
use internal::RawCache;



/// Structure for caching and looking up artifacts.
///
/// The `Cache` is the central structure of this crate. It helps to
/// avoid dependency duplication when multiple `Builder`s depend on the same
/// artifact.
///
/// Since all `Builder`s in the context of this crate are supposed to be wrapped
/// within `ArtifactPromise`s, the `Cache` is the only way of acquiring
/// an artifact in the first place.
///
/// Notice In the debugging version (when the **`diagnostics`** feature is active),
/// this struct contains a debuger `Doctor`, which
/// allows run-time inspection of various events.
/// In order to store it, the **`diagnostics`** `Cache` is generic to
/// some `Doctor`.
/// The `new()` method then returns a `Cache<NoopDoctor>`
/// and `new_with_doctor()` returns some `Cache<Doc>`.
///
/// Only an `Cache<Doc>` with `Doc: Sized` can be store in variables.
/// However, since most of the code does not care about the concrete
/// `Doctor` the default generic is `dyn Doctor`, on which all other methods are
/// defined.
/// An `Cache<dyn Doctor>` can not be stored, but it can be passed
/// on by reference (e.g. as `&mut Cache`). This prevents the use of
/// additional generics in **`diagnostics`** mode, and allows to easier achive
/// compatibility between **`diagnostics`** and non-**`diagnostics`** mode.
/// To ease conversion between `Cache<Doc>` and
/// `Cache<dyn Doctor>` (aka `Cache`), all creatable
/// `Cache`s (i.e. not `Cache<dyn Doctor>`) implement `DerefMut`
/// to `Cache<dyn Doctor>`.
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
/// This is a simple type-def to Cache, which gurantees independent of
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
/// This is a simple type-def to Cache, which gurantees independent of
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

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			promise: &AP
		) -> Option<ArtCan::Bin>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				AP: Promise<B, BCan>  {

		self.inner.lookup(promise)
	}

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			promise: &AP
		) -> Option<&B::Artifact>
			where
				ArtCan: CanRef<B::Artifact>,
				AP: Promise<B, BCan>  {

		self.inner.lookup_ref(promise)
	}

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Option<&mut B::Artifact>
			where
				ArtCan: CanRefMut<B::Artifact>,
				AP: Promise<B, BCan>  {

		self.inner.lookup_mut(promise)
	}

	/// Get and cast a clone of the stored artifact if it exists.
	///
	pub fn lookup_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			promise: &AP
		) -> Option<B::Artifact>
			where
				ArtCan: CanRef<B::Artifact>,
				B::Artifact: Clone,
				AP: Promise<B, BCan>  {

		self.inner.lookup_cloned(promise)
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
	/// If all strong references of the respective builder are out of scope, the
	/// `garbage_collection()` method can be used to get rid of the cached
	/// promise including the possibly still cached artifact and dyn state.
	///
	pub fn get<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Result<ArtCan::Bin, B::Err>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				AP: Promise<B, BCan>  {

		self.inner.get(promise)
	}

	/// Gets a reference to the artifact of the given builder.
	///
	pub fn get_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Result<&B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				AP: Promise<B, BCan>  {

		self.inner.get_ref(promise)
	}

	/// Gets a mutable reference to the artifact of the given builder.
	///
	/// As opposed to `get_ref`, this method will invalidate all dependents of
	/// the given builder.
	///
	pub fn get_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Result<&mut B::Artifact, B::Err>
			where
				ArtCan: CanRefMut<B::Artifact>,
				AP: Promise<B, BCan>  {

		self.inner.get_mut(promise)
	}

	/// Get a clone of the artifact of the given builder.
	///
	pub fn get_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Result<B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				B::Artifact: Clone,
				AP: Promise<B, BCan>  {

		self.inner.get_cloned(promise)
	}

	/// Gets the dynamic state of the given builder if any.
	///
	pub fn get_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self, promise: &AP
		) -> Option<&B::DynState>
			where
				AP: Promise<B, BCan>  {

		self.inner.get_dyn_state(promise)
	}

	/// Gets the dynamic state of the given builder, it will be initalized if
	/// it didn't exist yet.
	///
	pub fn dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self, promise: &AP
		) -> &B::DynState
			where
				AP: Promise<B, BCan>  {

		self.inner.dyn_state(promise)
	}

	/// Gets the mutable dynamic state of the given builder, it will be
	/// initalized if it didn't exist yet.
	/// 
	/// As opposed to `dyn_state`, this function will invalidate the given
	/// builder and cause its artifact to be rebuilded when next requested.
	///
	pub fn dyn_state_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self, promise: &AP
		) -> &mut B::DynState
			where
				AP: Promise<B, BCan>  {

		self.inner.dyn_state_mut(promise)
	}

	/// Deletes all cached artifacts in this cache, but keeps dynamic states.
	///
	pub fn clear_artifacts(&mut self) {
		self.inner.clear_artifacts()
	}

	/// Clears the entire cache including all kept promise, artifacts and
	/// dynamic states.
	///
	pub fn clear_all(&mut self) {
		self.inner.clear_all()
	}
	
	/// Deletes the artifact and the dynamic state of the given builder.
	/// 
	/// This function has the effect that all references of the given builder
	/// in this cache will be removed.
	/// 
	/// As a consequence, all dependent builders (if any) will be invalidated,
	/// but their dynamic states will be kept.
	/// 
	/// If you only want to remove the artifact see `invalidate()`.
	///
	pub fn purge<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		)
			where
				AP: Promise<B, BCan>  {

		self.inner.purge(promise)
	}

	/// Removes the given promise with its cached artifact from the cache and
	/// all depending artifacts (with their promises).
	///
	/// Depending artifacts are all artifacts which used the former during
	/// its building. The dependencies are automatically tracked via the
	/// `Resolver`.
	///
	pub fn invalidate<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		)
			where
				AP: Promise<B, BCan>  {

		self.inner.invalidate(promise)
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
		self.inner.garbage_collection()
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
		self.inner.number_of_known_builders()
	}
}





/// Resolves any `ArtifactPromise` to the artifact of the inner builder.
///
/// This struct is only available to `Builder`s within their `build()` method.
/// It gives certain access to the `Cache`, such as resolving
/// `ArtifactPromise`s.
///
/// The `Resolver` records each resolution of an `ArtifactPromise`
/// in order to keep track of dependencies between builders.
/// This dependency information is used for correct invalidation of dependants
/// on cache invalidation via `Cache::invalidate()`.
///
pub struct Resolver<'a, ArtCan, BCan: CanStrong, Doc = ()> {
	user: &'a BuilderEntry<BCan>,
	cache: &'a mut RawCache<ArtCan, BCan>,
	#[cfg(feature = "diagnostics")]
	diag_builder: &'a BuilderHandle<BCan>,
	_b: PhantomData<Doc>,
}

impl<'a, ArtCan: Debug, BCan: CanStrong + Debug, Doc: 'static> Resolver<'a, ArtCan, BCan, Doc> {

	fn track_dependency<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				B: Debug + 'static,
				AP: Promise<B, BCan> {

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


	/// Resolves the given `ArtifactPromise` into its artifact either by
	/// looking up the cached value in the associated `Cache` or by
	/// building it.
	///
	pub fn resolve<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<ArtCan::Bin, B::Err>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				B: Builder<ArtCan, BCan> + 'static,
				AP: Promise<B, BCan> {

		self.track_dependency(promise);
		self.cache.get(promise)
	}

	/// Resolves the given `ArtifactPromise` into its artifact reference either
	/// by looking up the cached value in the associated `Cache` or by
	/// building it.
	///
	pub fn resolve_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Result<&B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				AP: Promise<B, BCan>  {

		self.track_dependency(promise);
		self.cache.get_ref(promise)
	}

	/// Resolves the given `ArtifactPromise` into a clone of its artifact by
	/// using `resolve_ref()` and `clone().
	///
	pub fn resolve_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Result<B::Artifact, B::Err>
			where
				ArtCan: CanRef<B::Artifact>,
				B::Artifact: Clone,
				AP: Promise<B, BCan>  {

		self.track_dependency(promise);
		self.cache.get_cloned(promise)
	}

	/// Returns the dynamic state of the owning builder.
	///
	pub fn my_state(&mut self) -> &mut Doc {
		// The unwrap is safe here, because Cache ensures that a DynState exists
		// before we comme here.
		self.cache.dyn_state_cast_mut(self.user.id()).unwrap()
	}

	/// Get the dynamic static of given artifact promise.
	/// 
	/// See `my_state` to return the dynamic state of the current builder.
	///
	pub fn get_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
		&mut self,
		promise: &AP
	) -> &B::DynState
			where
				AP: Promise<B, BCan>, {

		self.track_dependency(promise);
		self.cache.dyn_state(promise)
	}
}


