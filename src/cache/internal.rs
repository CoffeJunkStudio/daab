


use std::any::Any;
use std::borrow::Borrow;
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
	id: BuilderId,
}

impl<BCan> BuilderEntry<BCan> {
	pub fn new<AP, B: ?Sized + 'static>(ap: &AP) -> Self
			where AP: Promise<B, BCan> {

		let id = ap.id();

		BuilderEntry {
			builder: ap.canned().can,
			id,
		}
	}

	pub fn id(&self) -> BuilderId {
		self.id
	}
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

impl<BCan> fmt::Pointer for BuilderEntry<BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", self.id.0)
	}
}



pub struct RawCache<
	ArtCan,
	BCan,
	#[cfg(feature = "diagnostics")] Doc: ?Sized = dyn Doctor<ArtCan, BCan>
> where BCan: CanStrong {

	/// Maps Builder-Capsules to their Artifact value
	artifacts: HashMap<BuilderId, ArtCan>,

	/// Maps Builder-Capsules to their DynState value
	dyn_states: HashMap<BuilderId, Box<dyn Any>>,

	/// Tracks the direct promise dependents of each promise
	dependents: HashMap<BuilderId, HashSet<BuilderId>>,

	/// Keeps a weak reference to all known builder ids that are those used in
	/// `cache` and/or dyn_state.
	know_builders: HashMap<BuilderId, <BCan as CanStrong>::CanWeak>,

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
					know_builders: HashMap::new(),

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
					know_builders: HashMap::new(),
				}
			}
		}
	}
}

/// Auxiliary function to casts an `Option` of `Box` of `Any` to `Doc`.
///
/// Must only be used with the correct `Doc`, or panics.
///
fn cast_dyn_state<Doc: 'static>(v: Option<Box<dyn Any>>) -> Option<Box<Doc>> {
	v.map(
		|b| {
			// Ensure value type
			b.downcast()
				.expect("Cached Builder DynState is of invalid type")
		}
	)
}

impl<ArtCan, BCan> RawCache<ArtCan, BCan>
		where ArtCan: Debug, BCan: CanStrong + Debug {

	/// Record the dependency of `user` upon `promise`.
	///
	pub fn track_dependency<AP, B: ?Sized>(
			&mut self,
			user: &BuilderEntry<BCan>,
			#[cfg(feature = "diagnostics")]
			diag_builder: &BuilderHandle<BCan>,
			promise: &AP
		)
			where
				B: Debug + 'static,
				AP: Promise<B, BCan> {


		let deps = self.get_dependents(promise);
		deps.insert(user.id());

		#[cfg(feature = "diagnostics")]
		self.doctor.resolve(diag_builder, &BuilderHandle::new(promise.clone()));

	}

	/// Returns the vector of dependents of promise
	///
	fn get_dependents<AP, B: ?Sized>(
			&mut self,
			promise: &AP
		) -> &mut HashSet<BuilderId>
			where
				AP: Promise<B, BCan> {


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
				AP: Promise<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.artifacts.get(&builder.id()).map(
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
				AP: Promise<B, BCan>  {


		// Get the artifact from the hash map ensuring integrity
		self.artifacts.get(&builder.id()).map(
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
				AP: Promise<B, BCan>  {

		let id = builder.id();

		// Since the user chose to use `mut` instead of `ref` he intends to
		// modify the artifact consequently invalidating all dependent builders
		self.invalidate_dependents(&id);

		// Get the artifact from the hash map ensuring integrity
		self.artifacts.get_mut(&id).map(
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
				AP: Promise<B, BCan>  {


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
				AP: Promise<B, BCan>  {


		self.make_builder_known(promise);

		let ent = BuilderEntry::new(promise);

		#[cfg(feature = "diagnostics")]
		let diag_builder = BuilderHandle::new(promise.clone());

		let art = promise.builder().builder.build(
			&mut Resolver {
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

		// keep id
		let id = promise.id();

		// Insert artifact
		self.artifacts.insert(
			id,
			art_can,
		);

		// Just unwrap, since we just inserted it
		self.artifacts.get_mut(&id).unwrap()

	}

	/// Gets the artifact of the given builder.
	///
	pub fn get<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		) -> ArtCan::Bin
			where
				ArtCan: CanSized<B::Artifact>,
				ArtCan: Clone,
				AP: Promise<B, BCan>  {


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
				AP: Promise<B, BCan>  {


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
				AP: Promise<B, BCan>  {


		if self.lookup_mut(promise).is_some() {
			// Here, requires a second look up because due to the build in the
			// else case, an `if let Some(_)` won't work due to lifetime issues
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
				AP: Promise<B, BCan>  {

		self.get_ref(promise).clone()
	}


	/// Get and cast the dynamic static of given builder id.
	///
	/// `T` must be the type of the respective dynamic state of `bid`,
	/// or this panics.
	///
	pub fn get_dyn_state_cast<T: 'static>(&mut self, bid: &BuilderId) -> Option<&mut T> {

		self.dyn_states.get_mut(bid)
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
	pub fn get_dyn_state_mut<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self, promise: &AP
		) -> Option<&mut B::DynState>
			where
				AP: Promise<B, BCan>  {

		// Since the user choses `mut` he intends to modify the dyn state this
		// requires the rebuild the artifact
		self.invalidate(promise);

		self.get_dyn_state_cast(&promise.id())
	}

	/// Gets the dynamic state of the given builder.
	///
	pub fn get_dyn_state<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self, promise: &AP
		) -> Option<&B::DynState>
			where
				AP: Promise<B, BCan>  {

		self.get_dyn_state_cast(&promise.id())
		// Demote `&mut` to `&`
		.map(|s| &*s)
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
				AP: Promise<B, BCan>  {

		self.make_builder_known(promise);

		cast_dyn_state(
			self.dyn_states.insert(promise.id(), Box::new(user_data))
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
				AP: Promise<B, BCan>  {

		let bid = promise.id();

		// Remove weak reference if no builder exists
		if !self.artifacts.contains_key(&bid) {
			self.know_builders.remove(&bid);
		}

		cast_dyn_state(
			self.dyn_states.remove(&bid)
		)
	}

	/// Deletes all dynamic states of this cache.
	///
	pub fn clear_artifacts(&mut self) {
		self.artifacts.clear();
		self.dependents.clear();

		// Remove weak reference for those without dyn state
		self.cleanup_unused_weak_refs();
	}

	/// Clears the entire cache including all kept promise, artifacts and
	/// dynamic states.
	///
	pub fn clear_all(&mut self) {
		self.artifacts.clear();
		self.dyn_states.clear();
		self.dependents.clear();
		self.know_builders.clear();

		#[cfg(feature = "diagnostics")]
		self.doctor.clear();
	}

	/// Auxiliary invalidation function using an untyped (aka `dyn Any`)
	/// `BuilderId`.
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

		self.artifacts.remove(builder);
	}

	/// Auxiliary invalidation function using an untyped (aka `dyn Any`)
	/// `BuilderId`, only invalidates dependents not the given build itself.
	///
	fn invalidate_dependents(&mut self, builder: &BuilderId) {
		if let Some(set) = self.dependents.remove(builder) {
			for dep in set {
				self.invalidate_any(&dep);
			}
		}
	}

	/// Removes the given promise with its cached artifact from the cache and
	/// all depending artifacts (with their promises).
	///
	pub fn invalidate<AP, B: ?Sized + Builder<ArtCan, BCan> + 'static>(
			&mut self,
			promise: &AP
		)
			where
				AP: Promise<B, BCan>  {


		self.invalidate_any(&promise.id());

		#[cfg(feature = "diagnostics")]
		self.doctor.invalidate(&BuilderHandle::new(promise));

		// Remove weak reference for those without dyn_state
		self.cleanup_unused_weak_refs();
	}

	/// Invalidates all builders and their dyn state which can not be builded
	/// any more, because there are no more references to them.
	///
	pub fn garbage_collection(&mut self) {

		let unreachable_builder_ids: Vec<_> = self.know_builders.iter()
			.filter(|(_bid, weak)| BCan::upgrade_from_weak(&weak).is_none())
			.map(|(bid, _weak)| *bid)
			.collect();

		for bid in unreachable_builder_ids {
			self.invalidate_any(&bid);
			self.dyn_states.remove(&bid);
			self.know_builders.remove(&bid);
		}
	}

	/// Remove any weak builder reference that is no longer used.
	fn cleanup_unused_weak_refs(&mut self) {

		let unused_builder_ids: Vec<_> = self.know_builders.keys().filter(|b|
			!(self.artifacts.contains_key(*b) || self.dyn_states.contains_key(*b))
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
				AP: Promise<B, BCan>  {

		let bid = promise.id();

		if !self.know_builders.contains_key(&bid) {
			self.know_builders.insert(bid, promise.canned().can.downgrade());
		}
	}

	/// Returns the number of currently kept artifact promises.
	///
	pub fn number_of_known_builders(&self) -> usize {
		self.know_builders.len()
	}
}

