


use std::any::Any;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;

use cfg_if::cfg_if;

use crate::CanStrong;
use crate::CanSized;
use crate::CanRef;
use crate::CanRefMut;

use crate::Promise;

use crate::Builder;
use crate::BuilderId;

use super::Resolver;



/// Auxiliary struct fro the `Cache` containing an untyped (aka
/// `dyn Any`) ArtifactPromise.
///
#[derive(Clone, Debug)]
pub(crate) struct BuilderEntry<BCan> {
	builder: BCan,
}

impl<BCan: CanStrong> BuilderEntry<BCan> {
	/// Constructs a new entry from given Promise.
	///
	pub(crate) fn new<AP, B: ?Sized + 'static>(ap: &AP) -> Self
			where AP: Promise<B, BCan> {

		BuilderEntry {
			builder: ap.canned().can,
		}
	}

	/// Returns id of this entry.
	///
	/// The id uniquely identifies the underlying builder.
	///
	pub(crate) fn id(&self) -> BuilderId {
		BuilderId::new(self.builder.can_as_ptr())
	}
}

impl<BCan: CanStrong> Hash for BuilderEntry<BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl<BCan: CanStrong> PartialEq for BuilderEntry<BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id().eq(&other.id())
	}
}

impl<BCan: CanStrong> Eq for BuilderEntry<BCan> {
}

impl<BCan: CanStrong> fmt::Pointer for BuilderEntry<BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", self.id().0)
	}
}



/// The raw cache. Only for internal use.
///
/// This struct is used by the "outer" Cache and Resolver.
/// Both require some of the internal-only functions of this type to provide the
/// outer interface.
///
/// When ever an id is used in any mapping here, its builder must be present in
/// the `known_builders` map.
///
pub(crate) struct RawCache<
	ArtCan,
	BCan,
	#[cfg(feature = "diagnostics")] Doc: ?Sized = dyn Doctor<ArtCan, BCan>
> where
		BCan: CanStrong {

	/// Maps builder id to their Artifact can.
	///
	artifacts: HashMap<BuilderId, ArtCan>,

	/// Maps builder id to their DynState value.
	///
	dyn_states: HashMap<BuilderId, Box<dyn Any>>,

	/// Tracks the set of direct depending builders of each builder, by id.
	///
	/// A dependent builder is one that requires the former's artifact to
	/// produce its own. This maps for each builder (key), which other
	/// builders (value) depend on it. I.e. it maps what artifacts needs to be
	/// invalidate if the former one becomes invalid.
	///
	/// A reverse mapping is provided via `dependencies`. Both must be kept in
	/// sync.
	///
	dependents: HashMap<BuilderId, HashSet<BuilderId>>,

	/// Tracks the set of direct dependencies of any builders, by id.
	///
	/// A dependency is a requirement for an builders artifact. This maps for
	/// each builder (key), which other builders (value) have been used to
	/// produce the former's artifact. I.e. it maps what dependencies must be
	/// removed if the former one becomes invalid (because that it does no
	/// longer depend on it).
	///
	/// This is the reverse of `dependents`. Both must be kept in sync.
	///
	dependencies: HashMap<BuilderId, HashSet<BuilderId>>,

	/// Keeps a weak reference to all known builders that are those which are
	/// used as builder id in any other mapping.
	///
	known_builders: HashMap<BuilderId, <BCan as CanStrong>::CanWeak>,

	/// The doctor for error diagnostics.
	#[cfg(feature = "diagnostics")]
	pub(crate) doctor: Doc,
}

cfg_if! {
	if #[cfg(feature = "diagnostics")] {
		use crate::Doctor;
		use crate::ArtifactHandle;
		use crate::BuilderHandle;

		impl<ArtCan, BCan, Doc> Debug for RawCache<ArtCan, BCan, Doc>
			where
				ArtCan: Debug,
				BCan: CanStrong + Debug,
				Doc: Debug {

			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "Cache {{ cache: {:?}, dependents: {:?}, doctor: {:?}, ... }}",
					self.artifacts, self.dependents, self.doctor)
			}
		}

		impl<ArtCan, BCan, Doc> RawCache<ArtCan, BCan, Doc>
			where BCan: CanStrong, Doc: Doctor<ArtCan, BCan> + 'static {

			/// Creates new empty cache with given doctor for inspection.
			///
			/// **Notice: This function is only available if the `diagnostics` feature has been activated**.
			///
			pub(crate) fn new_with_doctor(doctor: Doc) -> Self {
				Self {
					artifacts: HashMap::new(),
					dyn_states: HashMap::new(),
					dependents: HashMap::new(),
					dependencies: HashMap::new(),
					known_builders: HashMap::new(),

					doctor,
				}
			}
		}


	} else {

		impl<ArtCan, BCan> Debug for RawCache<ArtCan, BCan>
			where ArtCan: Debug, BCan: CanStrong + Debug {

			fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
				write!(f, "Cache {{ cache: {:?}, dependents: {:?}, ... }}",
					self.artifacts, self.dependents)
			}
		}

		impl<ArtCan, BCan> RawCache<ArtCan, BCan>
			where BCan: CanStrong {

			/// Creates a new empty cache.
			///
			pub(crate) fn new() -> Self {
				Self {
					artifacts: HashMap::new(),
					dyn_states: HashMap::new(),
					dependents: HashMap::new(),
					dependencies: HashMap::new(),
					known_builders: HashMap::new(),
				}
			}
		}
	}
}

impl<ArtCan, BCan> RawCache<ArtCan, BCan>
		where
			ArtCan: Debug,
			BCan: CanStrong {

	/// Record the dependency of `user` upon `promise`.
	///
	/// The `user` must be already listed in `known_builders`.
	///
	pub(super) fn track_dependency<AP, B: ?Sized>(
			&mut self,
			user: &BuilderEntry<BCan>,
			#[cfg(feature = "diagnostics")]
			diag_builder: &BuilderHandle<BCan>,
			promise: &AP
		)
			where
				B: Debug + 'static,
				AP: Promise<B, BCan> {

		// Ensure that the given promise is known.
		// User must exist already by contract.
		self.make_builder_known(promise);
		debug_assert!(self.is_builder_known_by_id(user.id()),
			"Tracking dependency for unknown builder");

		// Map dependents (`promise` has new dependent `user`)
		self.dependents.entry(promise.id())
			.or_insert_with(HashSet::new)
			.insert(user.id());

		// Revers mapping (`user` depends on `promise`)
		self.dependencies.entry(user.id())
			.or_insert_with(HashSet::new)
			.insert(promise.id());

		// Diagnostics
		#[cfg(feature = "diagnostics")]
		self.doctor.resolve(diag_builder, &BuilderHandle::new(promise.clone()));

	}


	/// Tests whether there exists an artifact for the given promise in this
	/// cache.
	///
	/// This function is equivalent to calling `is_some()` on any of the
	/// `lookup*` functions, but this one does no cast and has fewer
	/// generic requirements.
	///
	pub(crate) fn contains_artifact<AP: ?Sized, B: ?Sized>(
			&self,
			promise: &AP
		) -> bool
			where
				AP: Promise<B, BCan> {

		self.artifacts.contains_key(&promise.id())
	}

	/// Tests whether the artifact or dyn state of the given builder is
	/// recorded in this cache.
	///
	pub(crate) fn is_builder_known<AP: ?Sized, B: ?Sized>(
			&self,
			promise: &AP
		) -> bool
			where
				AP: Promise<B, BCan> {

		self.is_builder_known_by_id(promise.id())
	}

	/// Auxillary function to test whether given builder id is contained in
	/// `known_builders`.
	///
	fn is_builder_known_by_id(
			&self,
			bid: BuilderId,
		) -> bool {

		self.known_builders.contains_key(&bid)
	}

	/// Get the stored artifact by its bin if it exists.
	///
	pub(crate) fn lookup<AP, B: ?Sized>(
			&self,
			promise: &AP
		) -> Option<ArtCan::Bin>
			where
				B: Builder<ArtCan, BCan>,
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				AP: Promise<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.artifacts.get(&promise.id()).map(
			|ent| {
				// Ensure that the builder to the artifact is known
				debug_assert!(self.is_builder_known(promise),
					"Found artifact, but the builder is not known.");

				// Ensure value type
				ent.clone().downcast_can()
					.expect("Cached artifact is of invalid type")
			}
		)
	}

	/// Get the stored artifact by reference if it exists.
	///
	pub(crate) fn lookup_ref<AP, B: ?Sized>(
			&self,
			promise: &AP
		) -> Option<&B::Artifact>
			where
				B: Builder<ArtCan, BCan>,
				ArtCan: CanRef<B::Artifact>,
				AP: Promise<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.artifacts.get(&promise.id()).map(
			|ent| {
				// Ensure that the builder to the artifact is known
				debug_assert!(self.is_builder_known(promise),
					"Found artifact, but the builder is not known.");

				// Ensure value type
				ent.downcast_can_ref()
					.expect("Cached artifact is of invalid type")
			}
		)
	}

	/// Get the stored artifact by mutable reference if it exists.
	///
	pub(crate) fn lookup_mut<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Option<&mut B::Artifact>
			where
				B: Builder<ArtCan, BCan>,
				ArtCan: CanRefMut<B::Artifact>,
				AP: Promise<B, BCan>  {

		let id = promise.id();

		// Since the user chose to use `mut` instead of `ref` he intends to
		// modify the artifact consequently invalidating all dependent builders
		// TODO reconsider where the automatic invalidation is such a good idea
		self.invalidate_dependents(&id);

		// If an artifact exists, ensure that the builder is known too.
		debug_assert!(
			!self.contains_artifact(promise)
				|| self.is_builder_known(promise),
			"Found artifact, but the builder is not known."
		);

		// Get the artifact from the hash map ensuring integrity
		self.artifacts.get_mut(&id).map(
			|ent| {
				// Ensure value type
				ent.downcast_can_mut()
					.expect("Cached artifact is of invalid type")
			}
		)
	}

	/// Get a clone of the stored artifact if it exists.
	///
	pub(crate) fn lookup_cloned<AP, B: ?Sized>(
			&self,
			promise: &AP
		) -> Option<B::Artifact>
			where
				B: Builder<ArtCan, BCan>,
				B::Artifact: Clone,
				ArtCan: CanRef<B::Artifact>,
				AP: Promise<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.lookup_ref(promise).cloned()
	}


	/// Build and insert the artifact for `promise`.
	///
	/// This is an internal function.
	///
	/// There must be no artifact in cache for the given builder.
	///
	fn build<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<&mut ArtCan, B::Err>
			where
				B: Builder<ArtCan, BCan>,
				ArtCan: CanSized<B::Artifact>,
				AP: Promise<B, BCan>  {

		// Ensure that there yet is no artifact for that builder in cache
		debug_assert!(!self.contains_artifact(promise));

		// Ensure that the promise is known, because we will add its dynamic
		// state & (possibly) its artifact.
		self.make_builder_known(promise);

		// Ensure there is a DynState
		self.ensure_dyn_state(promise);

		// Create Resolver prerequisites
		let ent = BuilderEntry::new(promise);
		#[cfg(feature = "diagnostics")]
		let diag_builder = BuilderHandle::new(promise.clone());

		// Create a temporary resolver
		let mut resolver = Resolver {
			user: &ent,
			cache: self,
			#[cfg(feature = "diagnostics")]
			diag_builder: &diag_builder,
			_b: PhantomData,
		};

		// Construct the artifact
		let art_res = promise.builder().builder.build(
			&mut resolver,
		);

		// Add artifact to cache if it was successful, otherwise just return
		// the error
		art_res.map(move |art| {
			let art_bin = ArtCan::into_bin(art);

			// diagnostics
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

			// keep the id
			let id = promise.id();

			// Insert/Replace artifact
			self.artifacts.insert(
				id,
				art_can,
			);
			//.expect_none("Built an artifact while it was still in cache");

			// Just unwrap, since we just inserted it
			self.artifacts.get_mut(&id).unwrap()
		})

	}


	/// Gets the bin with the artifact of the given builder.
	///
	pub(crate) fn get<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<ArtCan::Bin, B::Err>
			where
				B: Builder<ArtCan, BCan>,
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				AP: Promise<B, BCan>  {


		if let Some(art) = self.lookup(promise) {
			Ok(art)

		} else {
			self.build(promise).map(|art| {
				art.clone().downcast_can()
				.expect("Just build artifact is of invalid type")
			})
		}
	}

	/// Gets a reference to the artifact of the given builder.
	///
	pub(crate) fn get_ref<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<&B::Artifact, B::Err>
			where
				B: Builder<ArtCan, BCan>,
				ArtCan: CanRef<B::Artifact>,
				AP: Promise<B, BCan>  {


		if self.lookup_ref(promise).is_some() {
			// Here, requires a second look up because due to the build in the
			// else case, an `if let Some(_)` won't work due to lifetime issues
			Ok(self.lookup_ref(promise).unwrap())

		} else {
			self.build(promise).map(|art| {
				art.downcast_can_ref()
				.expect("Just build artifact is of invalid type")
			})
		}
	}

	/// Gets a mutable reference to the artifact of the given builder.
	///
	pub(crate) fn get_mut<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<&mut B::Artifact, B::Err>
			where
				B: Builder<ArtCan, BCan>,
				ArtCan: CanRefMut<B::Artifact>,
				AP: Promise<B, BCan>  {


		if self.lookup_mut(promise).is_some() {
			// Here, requires a second look up because due to the build in the
			// else case, an `if let Some(_)` won't work due to lifetime issues
			Ok(self.lookup_mut(promise).unwrap())

		} else {
			self.build(promise).map(|art| {
				art.downcast_can_mut()
				.expect("Just build artifact is of invalid type")
			})
		}
	}

	/// Get a clone of the artifact of the given builder.
	///
	pub(crate) fn get_cloned<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> Result<B::Artifact, B::Err>
			where
				B: Builder<ArtCan, BCan>,
				B::Artifact: Clone,
				ArtCan: CanRef<B::Artifact>,
				AP: Promise<B, BCan>  {

		self.get_ref(promise).map(|art| {
			art.clone()
		})
	}


	/// Ensure given dyn state exists and return it by reference.
	///
	fn ensure_dyn_state<AP, B: ?Sized>(
			&mut self, promise: &AP
		) -> &mut B::DynState
			where
				B: Builder<ArtCan, BCan>,
				AP: Promise<B, BCan> {

		self.make_builder_known(promise);

		self.dyn_states
			.entry(promise.id())
			// Access entry or insert it with builder's default
			.or_insert_with(
				|| Box::new(promise.builder().builder.init_dyn_state())
			)
			// Ensure state type, it's safe because we have the builder's AP
			.downcast_mut()
			.expect("Cached dyn state is of invalid type")
	}


	/// Auxillary to get and cast the dynamic state of given builder id by
	/// mutable reference.
	///
	/// **This function is only intended for internal use, where the builder id
	/// has been carefully chosen**.
	///
	/// `T` must be the correct type of the dynamic state of `bid`,
	/// or this panics.
	///
	pub(super) fn dyn_state_cast_mut<T: 'static>(
			&mut self,
			bid: BuilderId
		) -> Option<&mut T> {

		// If dyn state exists, ensure that the builder to the dyn state is
		// known too.
		debug_assert!(
			!self.dyn_states.contains_key(&bid)
			|| self.is_builder_known_by_id(bid),
				"Found dyn state, but the builder is not known.");

		self.dyn_states.get_mut(&bid)
		.map(
			|b| {

				// Ensure state type, might fail if given wrong argument
				b.downcast_mut()
					.expect("Cached dyn state is of invalid type")
			}
		)
	}

	/// Auxillary to get and cast the dynamic state of given builder id by
	/// shared reference.
	///
	/// **This function is only intended for internal use, where the builder id
	/// has been carefully chosen**.
	///
	/// `T` must be the correct type of the dynamic state of `bid`,
	/// or this panics.
	///
	pub(super) fn dyn_state_cast_ref<T: 'static> (
			&self,
			bid: BuilderId
		) -> Option<&T> {

		self.dyn_states.get(&bid).map(
			|b| {
				// Ensure that the builder to the dyn state is known
				debug_assert!(self.is_builder_known_by_id(bid),
					"Found dyn state, but the builder is not known.");

				// Ensure value type
				b.downcast_ref()
					.expect("Cached dyn state is of invalid type")
			}
		)
	}

	/// Gets the mutable dynamic state of the given builder and invalidate it.
	///
	pub(crate) fn dyn_state_mut<AP, B: ?Sized>(
			&mut self, promise: &AP
		) -> &mut B::DynState
			where
				B: Builder<ArtCan, BCan>,
				AP: Promise<B, BCan> {

		// Since the user choses `mut` he intends to modify the dyn state this
		// requires the rebuild the artifact.
		// It is reasonable to invalidate it early as the cache is mutable
		// bounded through the returned reference, so no intermediate rebuild
		// can happen.
		self.invalidate(promise);

		self.ensure_dyn_state(promise)
	}

	/// Gets the dynamic state of the given builder.
	///
	pub(crate) fn dyn_state<AP, B: ?Sized>(
			&mut self, promise: &AP
		) -> &B::DynState
			where
				B: Builder<ArtCan, BCan>,
				AP: Promise<B, BCan>  {

		// Here, no invalidation, because we do not allow the user to modify the
		// dyn state.

		// Coerce to shared ref (`&`) and return
		self.ensure_dyn_state(promise)
	}

	/// Gets the dynamic state of the given builder, if it exists.
	///
	pub(crate) fn get_dyn_state<AP, B: ?Sized>(
			&self, promise: &AP
		) -> Option<&B::DynState>
			where
				B: Builder<ArtCan, BCan>,
				AP: Promise<B, BCan>  {

		self.dyn_state_cast_ref(promise.id())
	}

	/// Deletes the artifact and dynamic state of the given builder.
	///
	pub(crate) fn purge<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				B: Debug + 'static,
				AP: Promise<B, BCan>  {

		let bid = promise.id();

		// Remove weak reference of builder since we will remove all references
		// to it
		self.known_builders.remove(&bid);

		// Purge artifact & dyn state
		self.artifacts.remove(&bid);
		self.dyn_states.remove(&bid);

		// Invalidate dependents
		self.invalidate_by_id(&promise.id());

		#[cfg(feature = "diagnostics")]
		self.doctor.invalidate(&BuilderHandle::new(promise));
	}

	/// Deletes all artifacts of this cache.
	///
	pub(crate) fn clear_artifacts(&mut self) {
		self.artifacts.clear();
		self.dependents.clear();
		self.dependencies.clear();
	}

	/// Clears the entire cache including all kept promise, artifacts and
	/// dynamic states.
	///
	pub(crate) fn clear_all(&mut self) {
		self.artifacts.clear();
		self.dyn_states.clear();
		self.dependents.clear();
		self.dependencies.clear();
		self.known_builders.clear();

		#[cfg(feature = "diagnostics")]
		self.doctor.clear();
	}

	/// Auxiliary invalidation function using an untyped (aka `dyn Any`)
	/// `BuilderId`.
	///
	fn invalidate_by_id(&mut self, builder: &BuilderId) {

		// Remember already processed builders, because they have no more
		// dependencies mapping.
		let mut processed = HashSet::new();
		processed.insert(*builder);

		// Stack of builder to be invalidated.
		let mut pending = Vec::new();
		pending.push(*builder);


		while let Some(bid) = pending.pop() {
			// Mark builder as processed
			processed.insert(bid);

			// Get all dependents and invalidate them too
			if let Some(set) = self.dependents.remove(&bid) {
				for dep in set {
					pending.push(dep);
				}
			}

			// Remove dependencies too
			if let Some(set) = self.dependencies.remove(&bid) {
				for dep in set {
					// For each dependency ensure that either it had been
					// processed before, or it has a counterpart mapping.
					// In the latter case, remove the dependent relation.
					let found = processed.contains(&dep)
						|| self.dependents.get_mut(&dep)
							.expect("Mapped dependency has no dependents counterpart map.")
							.remove(&bid);

					// Notice the above code has important side-effects, thus
					// only the return value is tested in the assert macro.
					debug_assert!(found);
				}
			}


			self.artifacts.remove(&bid);

		}

	}

	/// Auxiliary invalidation function using an untyped (aka `dyn Any`)
	/// `BuilderId`, only invalidates dependents not the given build itself.
	///
	fn invalidate_dependents(&mut self, builder: &BuilderId) {
		if let Some(set) = self.dependents.remove(builder) {
			for dep in set {
				self.invalidate_by_id(&dep);
			}
		}
	}

	/// Removes the given promise with its cached artifact from the cache and
	/// all depending artifacts (with their promises).
	///
	pub(crate) fn invalidate<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				B: Debug + 'static,
				AP: Promise<B, BCan>  {


		self.invalidate_by_id(&promise.id());

		#[cfg(feature = "diagnostics")]
		self.doctor.invalidate(&BuilderHandle::new(promise));

	}

	/// Invalidates all builders and their dyn state which can not be builded
	/// any more, because there are no more references to them.
	///
	pub(crate) fn garbage_collection(&mut self) {

		let unreachable_builder_ids: Vec<_> = self.known_builders.iter()
			// Only retain those which can't be upgraded (i.e. no strong
			// references exist any more).
			.filter(|(_bid, weak)| BCan::upgrade_from_weak(&weak).is_none())
			.map(|(bid, _weak)| *bid)
			.collect();

		for bid in unreachable_builder_ids {
			self.invalidate_by_id(&bid);
			self.dyn_states.remove(&bid);
			self.known_builders.remove(&bid);
		}
	}

	/// Enlist given builder as known builder, that is to keep its weak
	/// reference while it is used in `cache` or `dyn_state`.
	fn make_builder_known<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				AP: Promise<B, BCan>  {

		let bid = promise.id();

		self.known_builders.entry(bid).or_insert_with(
			|| promise.canned().can.downgrade()
		);
	}

	/// Returns the number of currently kept artifact promises.
	///
	pub(crate) fn number_of_known_builders(&self) -> usize {
		self.known_builders.len()
	}
}



#[cfg(test)]
mod test {
	use super::*;
	use crate::prelude::*;
	use crate::Blueprint;
	use crate::test::*;
	use std::rc::Rc;
	use std::sync::Arc;

	fn init_entry() -> (BuilderEntry<Rc<dyn Any>>, BuilderId) {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::<_, Rc<dyn Any>>::new(builder);
		let id = bp.id();

		(BuilderEntry::new(&bp), id)
	}
	fn init_two_entries() -> (BuilderEntry<Rc<dyn Any>>, BuilderEntry<Rc<dyn Any>>) {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::<_, Rc<dyn Any>>::new(builder);

		(
			BuilderEntry::new(&bp),
			BuilderEntry::new(&bp.clone()),
		)
	}

	#[test]
	fn builder_entry_id() {
		let (entry, id) = init_entry();

		assert_eq!(entry.id(), id);
	}

	#[test]
	fn builder_entry_eq() {
		let (entry_one, entry_two) = init_two_entries();

		assert_eq!(entry_one.id(), entry_two.id());

		assert_eq!(entry_one, entry_two);
	}

	#[test]
	fn builder_entry_ne() {
		let (entry_one, _) = init_entry();
		let (entry_two, _) = init_entry();

		assert_ne!(entry_one.id(), entry_two.id());

		assert_ne!(entry_one, entry_two);
	}

	#[test]
	fn builder_entry_hash() {
		let (entry, id) = init_entry();

		let mut hasher1 = std::collections::hash_map::DefaultHasher::new();
		let mut hasher2 = std::collections::hash_map::DefaultHasher::new();

		entry.hash(&mut hasher1);
		id.hash(&mut hasher2);

		assert_eq!(hasher1.finish(), hasher2.finish());
	}


	cfg_if! {
		if #[cfg(feature = "diagnostics")] {
			use crate::diagnostics::NoopDoctor;
			fn new_cache_rc() -> RawCache<Rc<dyn Any>, Rc<dyn Any>, NoopDoctor> {
				RawCache::new_with_doctor(NoopDoctor::new())
			}
		}
		else {
			fn new_cache_rc() -> RawCache<Rc<dyn Any>, Rc<dyn Any>> {
				RawCache::new()
			}
		}
	}

	cfg_if! {
		if #[cfg(feature = "diagnostics")] {
			fn new_cache_box() -> RawCache<Box<dyn Any>, Arc<dyn Any + Send + Sync>, NoopDoctor> {
				RawCache::new_with_doctor(NoopDoctor::new())
			}
		}
		else {
			fn new_cache_box() -> RawCache<Box<dyn Any>, Arc<dyn Any + Send + Sync>> {
				RawCache::new()
			}
		}
	}

	fn ptr<T>(r: &T) -> *const T {
		r as *const T
	}

	#[test]
	fn contains_artifact() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);
		//let id = bp.id();

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(!cache.contains_artifact(&bp));

		cache.build(&bp).unpack();

		assert!(cache.contains_artifact(&bp));

		cache.invalidate(&bp);

		assert!(!cache.contains_artifact(&bp));
	}

	#[test]
	fn is_builder_known() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);
		//let id = bp.id();

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(!cache.is_builder_known(&bp));

		cache.build(&bp).unpack();

		assert!(cache.is_builder_known(&bp));

		cache.invalidate(&bp);

		assert!(cache.is_builder_known(&bp));

		cache.purge(&bp);

		assert!(!cache.is_builder_known(&bp));
	}

	#[test]
	fn is_builder_known_by_id() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);
		let id = bp.id();

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(!cache.is_builder_known_by_id(id));

		cache.dyn_state(&bp);

		assert!(cache.is_builder_known_by_id(id));

		cache.purge(&bp);

		assert!(!cache.is_builder_known_by_id(id));

		cache.invalidate(&bp);

		assert!(!cache.is_builder_known_by_id(id));

		cache.lookup(&bp);

		assert!(!cache.is_builder_known_by_id(id));
	}

	#[test]
	fn lookup() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.lookup(&bp).is_none());

		cache.dyn_state(&bp);

		assert!(cache.lookup(&bp).is_none());

		let value = cache.build(&bp).unpack().clone().downcast_can().unwrap();

		assert!(cache.lookup(&bp).is_some());
		assert_eq!(cache.lookup(&bp), cache.lookup(&bp));
		assert!(Rc::ptr_eq(&value, &cache.lookup(&bp).unwrap()));
		assert_eq!(Some(value), cache.lookup(&bp));

		cache.invalidate(&bp);

		assert!(cache.lookup(&bp).is_none());
	}

	#[test]
	fn lookup_ref() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.lookup_ref(&bp).is_none());

		cache.dyn_state(&bp);

		assert!(cache.lookup_ref(&bp).is_none());

		let value = cache.build(&bp).unpack().clone();
		let value = value.downcast_ref().unwrap();

		assert!(cache.lookup_ref(&bp).is_some());
		assert_eq!(cache.lookup_ref(&bp), cache.lookup_ref(&bp));
		assert_eq!(
			cache.lookup_ref(&bp).unwrap() as *const Leaf,
			cache.lookup_ref(&bp).unwrap() as *const Leaf,
		);
		assert_eq!(Some(value), cache.lookup_ref(&bp));

		cache.invalidate(&bp);

		assert!(cache.lookup_ref(&bp).is_none());
	}

	#[test]
	fn lookup_mut() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_box();
		let cache: &mut RawCache<Box<dyn Any>, Arc<dyn Any + Send + Sync>> = &mut cache_owned;

		assert!(cache.lookup_mut(&bp).is_none());

		cache.dyn_state(&bp);

		assert!(cache.lookup_mut(&bp).is_none());

		let mut value = cache.build(&bp).unpack().downcast_mut::<Leaf>().unwrap().clone();

		assert!(cache.lookup_mut(&bp).is_some());
		assert_eq!(
			cache.lookup_mut(&bp).unwrap().clone(),
			cache.lookup_mut(&bp).unwrap().clone(),
		);
		assert_eq!(
			cache.lookup_mut(&bp).unwrap() as *const Leaf,
			cache.lookup_mut(&bp).unwrap() as *const Leaf,
		);
		assert_eq!(Some(&mut value), cache.lookup_mut(&bp));

		cache.invalidate(&bp);

		assert!(cache.lookup_mut(&bp).is_none());
	}

	#[test]
	fn lookup_cloned() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_box();
		let cache: &mut RawCache<Box<dyn Any>, Arc<dyn Any + Send + Sync>> = &mut cache_owned;

		assert!(cache.lookup_cloned(&bp).is_none());

		cache.dyn_state(&bp);

		assert!(cache.lookup_cloned(&bp).is_none());

		let value = cache.build(&bp).unpack().downcast_ref::<Leaf>().unwrap().clone();

		assert!(cache.lookup_cloned(&bp).is_some());
		assert_eq!(cache.lookup_cloned(&bp), cache.lookup_cloned(&bp));
		assert_eq!(Some(value), cache.lookup_cloned(&bp));

		cache.invalidate(&bp);

		assert!(cache.lookup_cloned(&bp).is_none());
	}

	#[test]
	fn build() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(!cache.contains_artifact(&bp));
		assert!(!cache.dyn_states.contains_key(&bp.id()));

		let value_1 = cache.build(&bp).unpack().clone().downcast_can().unwrap();

		assert!(cache.contains_artifact(&bp));
		assert!(cache.dyn_states.contains_key(&bp.id()));

		assert!(Rc::ptr_eq(&value_1, &cache.lookup(&bp).unwrap()));

		cache.invalidate(&bp);
		assert!(!cache.contains_artifact(&bp));
		let value_2 = cache.build(&bp).unpack().clone().downcast_can().unwrap();
		assert!(cache.contains_artifact(&bp));

		assert!(Rc::ptr_eq(&value_2, &cache.lookup(&bp).unwrap()));

		assert!(!Rc::ptr_eq(&value_1, &value_2));
	}

	#[test]
	#[should_panic]
	fn build_double_build() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		// inverse assertion
		if cache.contains_artifact(&bp) {return}

		cache.build(&bp).unpack();

		// inverse assertion
		if !cache.contains_artifact(&bp) {return}

		// Now there is already an artifact, no new may be build.
		// This build should panic
		cache.build(&bp).unpack();
	}

	#[test]
	fn build_err() {
		let builder = BuilderLeafFallible::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(!cache.contains_artifact(&bp));

		// Set the builder to fail
		*cache.dyn_state_mut(&bp) = false;

		// Building must fail
		assert!(matches!(cache.build(&bp), Err(())));

		// No artifact will be recorded
		assert!(!cache.contains_artifact(&bp));

		// And it continues to fail
		assert!(matches!(cache.build(&bp), Err(())));
		assert!(!cache.contains_artifact(&bp));

		// Set the builder to produce an artifact
		*cache.dyn_state_mut(&bp) = true;

		// Now it will work
		assert!(cache.build(&bp).is_ok());
		assert!(cache.contains_artifact(&bp));

	}

	#[test]
	fn get() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(!cache.contains_artifact(&bp));

		let art = cache.get(&bp).unpack();
		assert_eq!(Some(art), cache.lookup(&bp));

		let art = cache.get(&bp).unpack();
		assert!(Rc::ptr_eq(&art, &cache.lookup(&bp).unwrap()));

		// Invalidate to retrieve a fresh artifact
		cache.invalidate(&bp);

		let art_n = cache.get(&bp).unpack();
		assert_ne!(art, art_n);
		assert!(!Rc::ptr_eq(&art, &art_n));

		assert_eq!(Some(art_n), cache.lookup(&bp));
	}

	#[test]
	fn get_ref() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(!cache.contains_artifact(&bp));

		let art = cache.get_ref(&bp).unpack().clone();
		assert_eq!(Some(&art), cache.lookup_ref(&bp));

		let art_ptr = ptr(cache.get_ref(&bp).unpack());
		assert_eq!(art_ptr, ptr(cache.lookup_ref(&bp).unwrap()));

		let old_art = cache.get(&bp).unpack();
		assert_eq!(ptr(old_art.as_ref()), art_ptr);

		// Invalidate to retrieve a fresh artifact
		cache.invalidate(&bp);

		let art_n_ptr = ptr(cache.get_ref(&bp).unpack());
		assert_ne!(old_art.as_ref(), unsafe{&(*art_n_ptr)});
		assert_ne!(ptr(old_art.as_ref()), art_n_ptr);

		assert_eq!(Some(art_n_ptr), cache.lookup_ref(&bp).map(|l| ptr(l)));
	}

	#[test]
	fn get_mut() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_box();
		let cache: &mut RawCache<Box<dyn Any>, Arc<dyn Any + Send + Sync>> = &mut cache_owned;

		assert!(!cache.contains_artifact(&bp));

		let mut art = cache.get_mut(&bp).unpack().clone();
		assert_eq!(Some(&mut art), cache.lookup_mut(&bp));

		let art_ptr = ptr(cache.get_mut(&bp).unpack());
		assert_eq!(art_ptr, ptr(cache.lookup_mut(&bp).unwrap()));

		let mut old_art = cache.get_cloned(&bp).unpack();

		// Invalidate to retrieve a fresh artifact
		cache.invalidate(&bp);

		let art_n = cache.get_mut(&bp).unpack();
		assert_ne!(&mut old_art, art_n);
	}

	#[test]
	fn get_cloned() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_box();
		let cache: &mut RawCache<Box<dyn Any>, Arc<dyn Any + Send + Sync>> = &mut cache_owned;

		assert!(!cache.contains_artifact(&bp));

		let art = cache.get_cloned(&bp).unpack();
		assert_eq!(Some(art), cache.lookup_cloned(&bp));

		let art = cache.get_cloned(&bp).unpack();
		assert_eq!(art, cache.lookup_cloned(&bp).unwrap());

		let old_art = cache.get_cloned(&bp).unpack();

		// Invalidate to retrieve a fresh artifact
		cache.invalidate(&bp);

		let art_n = cache.get_cloned(&bp).unpack();
		assert_ne!(old_art, art_n);
	}

	#[test]
	fn ensure_dyn_state() {
		let builder = BuilderLeafFallible::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.dyn_states.get(&bp.id()).is_none());

		let value = cache.ensure_dyn_state(&bp).clone();
		assert_eq!(value, *cache.dyn_state(&bp));

		// `ensure` may not change the value!
		assert_eq!(value, *cache.ensure_dyn_state(&bp));

		// Set the builder to fail
		let value = false;
		// This is different from the initial value
		assert_ne!(value, *cache.dyn_state(&bp));
		// Change dyn state
		*cache.dyn_state_mut(&bp) = value;

		// `ensure` may not change the value!
		assert_eq!(value, *cache.ensure_dyn_state(&bp));

	}

	#[test]
	fn dyn_state_cast_mut() {
		let builder = BuilderLeafFallible::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.dyn_states.get(&bp.id()).is_none());

		assert!(cache.dyn_state_cast_mut::<bool>(bp.id()).is_none());

		cache.ensure_dyn_state(&bp);

		assert!(cache.dyn_state_cast_mut::<bool>(bp.id()).is_some());

	}

	#[test]
	fn dyn_state_cast_ref() {
		let builder = BuilderLeafFallible::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.dyn_states.get(&bp.id()).is_none());

		assert!(cache.dyn_state_cast_ref::<bool>(bp.id()).is_none());

		cache.ensure_dyn_state(&bp);

		assert!(cache.dyn_state_cast_ref::<bool>(bp.id()).is_some());
	}

	#[test]
	fn dyn_state() {
		let builder = BuilderLeafFallible::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.dyn_states.get(&bp.id()).is_none());

		let dyn_state_ptr = ptr(cache.dyn_state(&bp));

		assert_eq!(dyn_state_ptr, ptr(cache.dyn_state(&bp)));
	}

	#[test]
	fn dyn_state_mut() {
		let builder = BuilderLeafFallible::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.dyn_states.get(&bp.id()).is_none());

		let dyn_state_ptr = ptr(cache.dyn_state_mut(&bp));

		assert_eq!(dyn_state_ptr, ptr(cache.dyn_state_mut(&bp)));
	}

	#[test]
	fn get_dyn_state() {
		let builder = BuilderLeafFallible::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		assert!(cache.dyn_states.get(&bp.id()).is_none());

		assert!(cache.get_dyn_state(&bp).is_none());

		let dyn_state_ptr = ptr(cache.ensure_dyn_state(&bp));

		assert_eq!(Some(dyn_state_ptr), cache.get_dyn_state(&bp).map(|d| ptr(d)));
	}

	#[test]
	fn purge() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		cache.get(&bp).unwrap();

		assert!(cache.is_builder_known(&bp));
		assert!(cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());

		cache.purge(&bp);

		assert!(!cache.is_builder_known(&bp));
		assert!(!cache.contains_artifact(&bp));
		assert!(!cache.get_dyn_state(&bp).is_some());

	}

	#[test]
	fn purge_deps() {
		let base_bp = Blueprint::new(BuilderLeafFallible::new());

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(base_bp.clone());
		let mid_bp = Blueprint::new(builder);

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(mid_bp.clone());
		let end_bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;


		cache.get(&end_bp).unwrap();

		assert!(cache.is_builder_known(&base_bp));
		assert!(cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());

		cache.purge(&mid_bp);

		assert!(cache.is_builder_known(&base_bp));
		assert!(cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());

		assert!(!cache.is_builder_known(&mid_bp));
		assert!(!cache.contains_artifact(&mid_bp));
		assert!(!cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(!cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());
	}

	#[test]
	fn clear_artifacts() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		cache.get(&bp).unwrap();

		assert!(cache.is_builder_known(&bp));
		assert!(cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());

		cache.clear_artifacts();

		assert!(cache.is_builder_known(&bp));
		assert!(!cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());

	}

	#[test]
	fn clear_artifacts_deps() {
		let base_bp = Blueprint::new(BuilderLeafFallible::new());

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(base_bp.clone());
		let mid_bp = Blueprint::new(builder);

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(mid_bp.clone());
		let end_bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;


		cache.get(&end_bp).unwrap();

		assert!(cache.is_builder_known(&base_bp));
		assert!(cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());

		cache.clear_artifacts();

		assert!(cache.is_builder_known(&base_bp));
		assert!(!cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(!cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(!cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());
	}

	#[test]
	fn clear_all() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		cache.get(&bp).unwrap();

		assert!(cache.is_builder_known(&bp));
		assert!(cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());

		cache.clear_all();

		assert!(!cache.is_builder_known(&bp));
		assert!(!cache.contains_artifact(&bp));
		assert!(!cache.get_dyn_state(&bp).is_some());

	}

	#[test]
	fn clear_all_deps() {
		let base_bp = Blueprint::new(BuilderLeafFallible::new());

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(base_bp.clone());
		let mid_bp = Blueprint::new(builder);

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(mid_bp.clone());
		let end_bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;


		cache.get(&end_bp).unwrap();

		assert!(cache.is_builder_known(&base_bp));
		assert!(cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());

		cache.clear_all();

		assert!(!cache.is_builder_known(&base_bp));
		assert!(!cache.contains_artifact(&base_bp));
		assert!(!cache.get_dyn_state(&base_bp).is_some());

		assert!(!cache.is_builder_known(&mid_bp));
		assert!(!cache.contains_artifact(&mid_bp));
		assert!(!cache.get_dyn_state(&mid_bp).is_some());

		assert!(!cache.is_builder_known(&end_bp));
		assert!(!cache.contains_artifact(&end_bp));
		assert!(!cache.get_dyn_state(&end_bp).is_some());
	}

	#[test]
	fn invalidate() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		cache.get(&bp).unwrap();

		assert!(cache.is_builder_known(&bp));
		assert!(cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());

		cache.invalidate(&bp);

		assert!(cache.is_builder_known(&bp));
		assert!(!cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());

	}

	#[test]
	fn invalidate_deps() {
		let base_bp = Blueprint::new(BuilderLeafFallible::new());

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(base_bp.clone());
		let mid_bp = Blueprint::new(builder);

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(mid_bp.clone());
		let end_bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;


		cache.get(&end_bp).unwrap();

		assert!(cache.is_builder_known(&base_bp));
		assert!(cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());

		cache.invalidate(&mid_bp);

		assert!(cache.is_builder_known(&base_bp));
		assert!(cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(!cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(!cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());
	}

	#[test]
	fn invalidate_deps_complex() {
		let base_bp_1 = Blueprint::new(BuilderLeafFallible::new());
		let base_bp_2 = Blueprint::new(BuilderLeafFallible::new());

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(base_bp_1.clone());
		let mid_bp = Blueprint::new(builder);

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(mid_bp.clone());
		let end_bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;


		// Build base layout
		cache.get(&end_bp).unwrap();

		assert!(cache.is_builder_known(&base_bp_1));
		assert!(cache.contains_artifact(&base_bp_1));
		assert!(cache.get_dyn_state(&base_bp_1).is_some());

		assert!(!cache.is_builder_known(&base_bp_2));
		assert!(!cache.contains_artifact(&base_bp_2));
		assert!(!cache.get_dyn_state(&base_bp_2).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());

		// Change dependency from base 1 to base 2
		// This must invalidate mid & end
		cache.dyn_state_mut(&mid_bp).0 = base_bp_2.clone();

		assert!(cache.is_builder_known(&base_bp_1));
		assert!(cache.contains_artifact(&base_bp_1));
		assert!(cache.get_dyn_state(&base_bp_1).is_some());

		assert!(!cache.is_builder_known(&base_bp_2));
		assert!(!cache.contains_artifact(&base_bp_2));
		assert!(!cache.get_dyn_state(&base_bp_2).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(!cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(!cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());

		// Build secondary layout
		cache.get(&end_bp).unwrap();

		assert!(cache.is_builder_known(&base_bp_1));
		assert!(cache.contains_artifact(&base_bp_1));
		assert!(cache.get_dyn_state(&base_bp_1).is_some());

		assert!(cache.is_builder_known(&base_bp_2));
		assert!(cache.contains_artifact(&base_bp_2));
		assert!(cache.get_dyn_state(&base_bp_2).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());

		// Only invalidate base 1 which has no more dependents!
		cache.invalidate(&base_bp_1);

		assert!(cache.is_builder_known(&base_bp_1));
		assert!(!cache.contains_artifact(&base_bp_1));
		assert!(cache.get_dyn_state(&base_bp_1).is_some());

		assert!(cache.is_builder_known(&base_bp_2));
		assert!(cache.contains_artifact(&base_bp_2));
		assert!(cache.get_dyn_state(&base_bp_2).is_some());

		assert!(cache.is_builder_known(&mid_bp));
		assert!(cache.contains_artifact(&mid_bp));
		assert!(cache.get_dyn_state(&mid_bp).is_some());

		assert!(cache.is_builder_known(&end_bp));
		assert!(cache.contains_artifact(&end_bp));
		assert!(cache.get_dyn_state(&end_bp).is_some());
	}

	#[test]
	fn garbage_collection() {
		let builder = BuilderLeaf::new();
		let bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;

		cache.get(&bp).unwrap();

		assert!(cache.is_builder_known(&bp));
		assert!(cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());
		assert_eq!(1, cache.number_of_known_builders());

		// While references are still reachable, GC may not change anything
		cache.garbage_collection();

		assert!(cache.is_builder_known(&bp));
		assert!(cache.contains_artifact(&bp));
		assert!(cache.get_dyn_state(&bp).is_some());
		assert_eq!(1, cache.number_of_known_builders());

		// Drop reference
		drop(bp);

		// Do GC, which now must remove the dropped reference
		cache.garbage_collection();

		assert_eq!(0, cache.number_of_known_builders());
	}

	#[test]
	fn garbage_collection_deps() {
		let base_bp = Blueprint::new(BuilderLeafFallible::new());

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(base_bp.clone());
		let mid_bp = Blueprint::new(builder);

		let builder = BuilderVariableNode::new::<Rc<dyn Any>, Rc<dyn Any>>(mid_bp.clone());
		let end_bp = Blueprint::new(builder);

		let mut cache_owned = new_cache_rc();
		let cache: &mut RawCache<Rc<dyn Any>, Rc<dyn Any>> = &mut cache_owned;


		cache.get(&end_bp).unwrap();

		assert_eq!(3, cache.number_of_known_builders());

		// Remove to mid & end
		drop(mid_bp);
		drop(end_bp);

		// Clean only mid & end
		cache.garbage_collection();
		// BuilderVariableNode requires additional GC cycles
		// because it stores APs in its dyn state!
		cache.garbage_collection();

		assert_eq!(1, cache.number_of_known_builders());

		// base is still reachable and must be retained
		assert!(cache.is_builder_known(&base_bp));
		assert!(cache.contains_artifact(&base_bp));
		assert!(cache.get_dyn_state(&base_bp).is_some());
	}

}



