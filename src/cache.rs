


use std::any::Any;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Debug;
use std::ops::Deref;
use std::ops::DerefMut;
use std::marker::PhantomData;

use cfg_if::cfg_if;

use crate::Can;
use crate::CanStrong;
use crate::CanSized;
use crate::CanRef;
use crate::CanRefMut;

use crate::ArtifactPromiseTrait;

use crate::Builder;
use crate::BuilderId;

mod internal;

use internal::BuilderEntry;





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
pub struct ArtifactCache<
	ArtCan,
	BCan,
	#[cfg(feature = "diagnostics")] T: ?Sized = dyn Doctor<ArtCan, BCan>
> where BCan: CanStrong {

	/// Maps Builder-Capsules to their Artifact value
	cache: HashMap<BuilderId, ArtCan>,

	/// Maps Builder-Capsules to their DynState value
	dyn_state: HashMap<BuilderId, Box<dyn Any>>,

	/// Tracks the direct promise dependents of each promise
	dependents: HashMap<BuilderId, HashSet<BuilderId>>,

	/// Keeps a weak reference to all known builder ids that are those used in
	/// `cache` and/or dyn_state.
	know_builders: HashMap<BuilderId, <BCan as CanStrong>::CanWeak>,

	/// The doctor for error diagnostics.
	#[cfg(feature = "diagnostics")]
	doctor: T,
}

cfg_if! {
	if #[cfg(feature = "diagnostics")] {
		use crate::Doctor;
		use crate::DefDoctor;
		use crate::ArtifactHandle;
		use crate::BuilderHandle;

		impl<ArtCan, BCan: CanStrong> Default for ArtifactCache<ArtCan, BCan, DefDoctor> {
			fn default() -> Self {
				ArtifactCache::new()
			}
		}

		pub type ArtifactCacheOwned<ArtCan, BCan> = ArtifactCache<ArtCan, BCan, DefDoctor>;

		impl<ArtCan: Debug, BCan: CanStrong + Debug, T: Debug> Debug for ArtifactCache<ArtCan, BCan, T> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "ArtifactCache {{ cache: {:?}, dependents: {:?}, doctor: {:?} }}",
					self.cache, self.dependents, self.doctor)
			}
		}

		impl<ArtCan, BCan: CanStrong> ArtifactCache<ArtCan, BCan, DefDoctor> {
			/// Creates a new empty cache with a dummy doctor.
			///
			pub fn new() -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependents: HashMap::new(),
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
					dependents: HashMap::new(),
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

		pub type ArtifactCacheOwned<ArtCan, BCan> = ArtifactCache<ArtCan, BCan>;

		impl<ArtCan: Debug, BCan: CanStrong + Debug> Debug for ArtifactCache<ArtCan, BCan> {
			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "ArtifactCache {{ cache: {:?}, dependents: {:?} }}",
					self.cache, self.dependents)
			}
		}

		impl<ArtCan, BCan: CanStrong> ArtifactCache<ArtCan, BCan> {
			/// Creates a new empty cache.
			///
			pub fn new() -> Self {
				Self {
					cache: HashMap::new(),
					dyn_state: HashMap::new(),
					dependents: HashMap::new(),
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
				AP: ArtifactPromiseTrait<B, BCan>  {


		let deps = self.get_dependants(promise);
		if !deps.contains(&user.id()) {
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
				ArtCan: CanRef<B::Artifact>,
				AP: ArtifactPromiseTrait<B, BCan>,  {


		let deps = self.get_dependants(promise);
		if !deps.contains(&user.id()) {
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
				AP: ArtifactPromiseTrait<B, BCan>  {


		if !self.dependents.contains_key(&promise.id()) {
			self.dependents.insert(promise.id(), HashSet::new());
		}

		self.dependents.get_mut(&promise.id()).unwrap()
	}

	/// Get and cast the stored artifact if it exists.
	///
	pub fn lookup<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&self,
			builder: &AP
		) -> Option<ArtCan::Bin>
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
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
				AP: ArtifactPromiseTrait<B, BCan>  {


		self.make_builder_known(promise);

		let ent = BuilderEntry::new(promise);

		#[cfg(feature = "diagnostics")]
		let diag_builder = BuilderHandle::new(promise.clone());

		let art = promise.builder().builder.build(
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
				ArtCan: CanRef<B::Artifact>,
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
				ArtCan: CanRefMut<B::Artifact>,
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
				ArtCan: CanRef<B::Artifact>,
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
				AP: ArtifactPromiseTrait<B, BCan>  {


		self.get_dyn_state_cast(&promise.id())
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
		self.dependents.clear();
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

		if let Some(set) = self.dependents.remove(builder) {
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
				AP: ArtifactPromiseTrait<B, BCan>  {

		let bid = promise.id();

		if !self.know_builders.contains_key(&bid) {
			self.know_builders.insert(bid, promise.canned().can.downgrade());
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
				ArtCan: CanRef<B::Artifact>,
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
				ArtCan: CanRef<B::Artifact>,
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
				AP: ArtifactPromiseTrait<B, BCan>, {


		self.cache.get_dyn_state(promise)
	}
}


