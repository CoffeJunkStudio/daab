


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
pub struct BuilderEntry<BCan> {
	builder: BCan,
}

impl<BCan: CanStrong> BuilderEntry<BCan> {
	/// Constructs a new entry from given Promise.
	///
	pub fn new<AP, B: ?Sized + 'static>(ap: &AP) -> Self
			where AP: Promise<B, BCan> {

		BuilderEntry {
			builder: ap.canned().can,
		}
	}

	/// Returns id of this entry.
	///
	/// The id uniquely identifies the underlying builder.
	///
	pub fn id(&self) -> BuilderId {
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
pub struct RawCache<
	ArtCan,
	BCan,
	#[cfg(feature = "diagnostics")] Doc: ?Sized = dyn Doctor<ArtCan, BCan>
> where BCan: CanStrong {

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
	pub doctor: Doc,
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
			pub fn new_with_doctor(doctor: Doc) -> Self {
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
			pub fn new() -> Self {
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
		where ArtCan: Debug, BCan: CanStrong + Debug {

	/// Record the dependency of `user` upon `promise`.
	///
	/// The `user` must be already listed in `known_builders`.
	///
	pub(crate) fn track_dependency<AP, B: ?Sized>(
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
	pub fn contains_artifact<AP: ?Sized, B: ?Sized>(
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
	pub fn is_builder_known<AP: ?Sized, B: ?Sized>(
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
	pub fn lookup<AP, B: ?Sized>(
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

	/// Get and stored artifact by reference if it exists.
	///
	pub fn lookup_ref<AP, B: ?Sized>(
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
	pub fn lookup_mut<AP, B: ?Sized>(
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
	pub fn lookup_cloned<AP, B: ?Sized>(
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
	pub fn get<AP, B: ?Sized>(
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
	pub fn get_ref<AP, B: ?Sized>(
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
	pub fn get_mut<AP, B: ?Sized>(
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
	pub fn get_cloned<AP, B: ?Sized>(
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
	pub(crate) fn dyn_state_cast_mut<T: 'static>(
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
	pub(crate) fn dyn_state_cast_ref<T: 'static> (
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
	pub fn dyn_state_mut<AP, B: ?Sized>(
			&mut self, promise: &AP
		) -> &mut B::DynState
			where
				B: Builder<ArtCan, BCan> + 'static,
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
	pub fn dyn_state<AP, B: ?Sized>(
			&mut self, promise: &AP
		) -> &B::DynState
			where
				B: Builder<ArtCan, BCan> + 'static,
				AP: Promise<B, BCan>  {

		// Here, no invalidation, because we do not allow the user to modify the
		// dyn state.

		// Coerce to shared ref (`&`) and return
		self.ensure_dyn_state(promise)
	}

	/// Gets the dynamic state of the given builder, if it exists.
	///
	pub fn get_dyn_state<AP, B: ?Sized>(
			&self, promise: &AP
		) -> Option<&B::DynState>
			where
				B: Builder<ArtCan, BCan> + 'static,
				AP: Promise<B, BCan>  {

		self.dyn_state_cast_ref(promise.id())
	}

	/// Deletes the artifact and dynamic state of the given builder.
	///
	pub fn purge<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		)
			where
				B: Builder<ArtCan, BCan> + 'static,
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
	pub fn clear_artifacts(&mut self) {
		self.artifacts.clear();
		self.dependents.clear();
		self.dependencies.clear();
	}

	/// Clears the entire cache including all kept promise, artifacts and
	/// dynamic states.
	///
	pub fn clear_all(&mut self) {
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
	pub fn invalidate<AP, B: ?Sized>(
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
	pub fn garbage_collection(&mut self) {

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
	pub fn number_of_known_builders(&self) -> usize {
		self.known_builders.len()
	}
}


#[cfg(test)]
mod test {
	use super::*;
	use crate::Blueprint;
	use crate::test::*;
	use std::rc::Rc;

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

}



