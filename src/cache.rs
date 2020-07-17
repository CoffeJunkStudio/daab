


use std::borrow::Borrow;
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;

use cfg_if::cfg_if;

use crate::CanStrong;
use crate::CanSized;
use crate::CanRef;
use crate::CanRefMut;

use crate::ArtifactPromiseTrait;

use crate::Builder;

mod internal;

use internal::BuilderEntry;
use internal::RawArtifactCache;



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
/// and `new_with_doctor()` returns some `ArtifactCache<Doc>`.
///
/// Only an `ArtifactCache<Doc>` with `Doc: Sized` can be store in variables.
/// However, since most of the code does not care about the concrete
/// `Doctor` the default generic is `dyn Doctor`, on which all other methods are
/// defined.
/// An `ArtifactCache<dyn Doctor>` can not be stored, but it can be passed
/// on by reference (e.g. as `&mut ArtifactCache`). This prevents the use of
/// additional generics in **`diagnostics`** mode, and allows to easier achive
/// compatibility between **`diagnostics`** and non-**`diagnostics`** mode.
/// To ease conversion between `ArtifactCache<Doc>` and
/// `ArtifactCache<dyn Doctor>` (aka `ArtifactCache`), all creatable
/// `ArtifactCache`s (i.e. not `ArtifactCache<dyn Doctor>`) implement `DerefMut`
/// to `ArtifactCache<dyn Doctor>`.
///
pub struct ArtifactCache<
	ArtCan,
	BCan,
	#[cfg(feature = "diagnostics")] Doc: ?Sized = dyn Doctor<ArtCan, BCan>
> where BCan: CanStrong {

	/// The inner cache
	#[cfg(feature = "diagnostics")]
	inner: RawArtifactCache<ArtCan, BCan, Doc>,
	#[cfg(not(feature = "diagnostics"))]
	inner: RawArtifactCache<ArtCan, BCan>,

}

/// The ownable and storable variant of the ArtifactCache.
///
/// This is a simple type-def to ArtifactCache, which gurantees independent of
/// whether the `diagnostics` feature is enabled or not that this type is
/// constructable and storable as owned value.
///
/// When ever a ArtifactCache needs to be stored such as in a struct, this type
/// alias should be preferred over using ArtifactCache directly.
///
#[cfg(feature = "diagnostics")]
pub type ArtifactCacheOwned<ArtCan, BCan> =
	ArtifactCache<ArtCan, BCan, DefDoctor>;
#[cfg(not(feature = "diagnostics"))]
pub type ArtifactCacheOwned<ArtCan, BCan> =
	ArtifactCache<ArtCan, BCan>;

impl<ArtCan, BCan> Default for ArtifactCacheOwned<ArtCan, BCan>
	where BCan: CanStrong {

	fn default() -> Self {
		ArtifactCacheOwned::new()
	}
}

impl<ArtCan, BCan> ArtifactCacheOwned<ArtCan, BCan>
	where BCan: CanStrong {

	/// Creates a new empty cache with a dummy doctor.
	///
	pub fn new() -> Self {
		cfg_if! {
			if #[cfg(feature = "diagnostics")] {
				Self {
					inner: RawArtifactCache::new_with_doctor(Default::default())
				}
			} else {
				Self {
					inner: RawArtifactCache::new()
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

		impl<ArtCan, BCan, Doc> Debug for ArtifactCache<ArtCan, BCan, Doc>
			where ArtCan: Debug, BCan: CanStrong + Debug, Doc: Debug {

			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				self.inner.fmt(f)
			}
		}

		impl<ArtCan, BCan, Doc> ArtifactCache<ArtCan, BCan, Doc>
			where BCan: CanStrong, Doc: Doctor<ArtCan, BCan> + 'static {

			/// Creates new empty cache with given doctor for inspection.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn new_with_doctor(doctor: Doc) -> Self {
				Self {
					inner: RawArtifactCache::new_with_doctor(doctor)
				}
			}

			/// Returns a reference of the inner doctor.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn doctor(&mut self) -> &mut Doc {
				&mut self.inner.doctor
			}

			/// Consumes the `ArtifactCache` and returns the inner doctor.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub fn into_doctor(self) -> Doc {
				self.inner.doctor
			}
		}

		impl<ArtCan, BCan, Doc> Deref for ArtifactCache<ArtCan, BCan, Doc>
			where BCan: CanStrong, Doc: Doctor<ArtCan, BCan> + 'static {

			type Target = ArtifactCache<ArtCan, BCan>;

			fn deref(&self) -> &Self::Target {
				self
			}
		}

		impl<ArtCan, BCan, Doc> DerefMut for ArtifactCache<ArtCan, BCan, Doc>
			where BCan: CanStrong, Doc: Doctor<ArtCan, BCan> + 'static {

			fn deref_mut(&mut self) -> &mut Self::Target {
				self
			}
		}


	} else {
		impl<ArtCan, BCan> Debug for ArtifactCache<ArtCan, BCan>
			where ArtCan: Debug, BCan: CanStrong + Debug {

			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				self.inner.fmt(f)
			}
		}
	}
}

impl<ArtCan: Debug, BCan: CanStrong + Debug> ArtifactCache<ArtCan, BCan> {

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			promise: &AP
		) -> Option<ArtCan::Bin>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				AP: ArtifactPromiseTrait<B, BCan>  {

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
				AP: ArtifactPromiseTrait<B, BCan>  {

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
				AP: ArtifactPromiseTrait<B, BCan>  {

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
				AP: ArtifactPromiseTrait<B, BCan>  {

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
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.get(promise)
	}

	/// Gets a reference of the artifact of the given builder.
	///
	pub fn get_ref<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> &B::Artifact
			where
				ArtCan: CanRef<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.get_ref(promise)
	}

	/// Gets a mutable reference of the artifact of the given builder.
	///
	pub fn get_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> &mut B::Artifact
			where
				ArtCan: CanRefMut<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.get_mut(promise)
	}

	/// Get a clone of the artifact of the given builder.
	///
	pub fn get_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> B::Artifact
			where
				ArtCan: CanRef<B::Artifact>,
				B::Artifact: Clone,
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.get_cloned(promise)
	}

	/// Gets the dynamic state of the given builder.
	///
	pub fn get_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self, promise: &AP
		) -> Option<&B::DynState>
			where
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.get_dyn_state(promise)
	}

	/// Gets the dynamic state of the given builder.
	///
	pub fn get_dyn_state_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self, promise: &AP
		) -> Option<&mut B::DynState>
			where
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.get_dyn_state_mut(promise)
	}

	/// Sets the dynamic state of the given builder.
	///
	#[deprecated = "Will be removed soon"]
	pub fn set_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP,
			user_data: B::DynState
		) -> Option<Box<B::DynState>>
			where
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.set_dyn_state(promise, user_data)
	}

	/// Deletes the dynamic state of the given builder.
	///
	pub fn remove_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> Option<Box<B::DynState>>
			where
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.inner.remove_dyn_state(promise)
	}

	/// Deletes all dynamic states of this cache.
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
				AP: ArtifactPromiseTrait<B, BCan>  {

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
/// It gives certain access to the `ArtifactCache`, such as resolving
/// `ArtifactPromise`s.
///
/// The `ArtifactResolver` records each resolution of an `ArtifactPromise`
/// in order to keep track of dependencies between builders.
/// This dependency information is used for correct invalidation of dependants
/// on cache invalidation via `ArtifactCache::invalidate()`.
///
pub struct ArtifactResolver<'a, ArtCan, BCan: CanStrong, Doc = ()> {
	user: &'a BuilderEntry<BCan>,
	cache: &'a mut RawArtifactCache<ArtCan, BCan>,
	#[cfg(feature = "diagnostics")]
	diag_builder: &'a BuilderHandle<BCan>,
	_b: PhantomData<Doc>,
}

impl<'a, ArtCan: Debug, BCan: CanStrong + Debug, Doc: 'static> ArtifactResolver<'a, ArtCan, BCan, Doc> {

	fn track_dependency<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				B: Debug + 'static,
				AP: ArtifactPromiseTrait<B, BCan> {

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
	/// looking up the cached value in the associated `ArtifactCache` or by
	/// building it.
	///
	pub fn resolve<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> ArtCan::Bin
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				B: Builder<ArtCan, BCan> + 'static,
				AP: ArtifactPromiseTrait<B, BCan> {

		self.track_dependency(promise);
		self.cache.get(promise)
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
				ArtCan: CanRef<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.track_dependency(promise);
		self.cache.get_ref(promise)
	}

	/// Resolves the given `ArtifactPromise` into a clone of its artifact by
	/// using `resolve_ref()` and `clone().
	///
	pub fn resolve_cloned<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> B::Artifact
			where
				ArtCan: CanRef<B::Artifact>,
				B::Artifact: Clone,
				AP: ArtifactPromiseTrait<B, BCan>  {

		self.track_dependency(promise);
		self.cache.get_cloned(promise)
	}

	/// Returns the dynamic state of the owning builder.
	///
	/// ## Panic
	///
	/// This function panics if no dynamic state has been set for this builder.
	///
	pub fn my_state(&mut self) -> &mut Doc {
		self.cache.get_dyn_state_cast(self.user.borrow()).unwrap()
	}

	/// Gets the dynamic state of the owning builder.
	///
	pub fn get_my_state(&mut self) -> Option<&mut Doc> {
		self.cache.get_dyn_state_cast(self.user.borrow())
	}

	/// Get the dynamic static of given artifact promise.
	///
	pub fn get_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
		&mut self,
		promise: &AP
	) -> Option<&B::DynState>
			where
				AP: ArtifactPromiseTrait<B, BCan>, {

		self.track_dependency(promise);
		self.cache.get_dyn_state(promise)
	}
}


