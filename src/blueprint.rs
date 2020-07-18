


use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;

use cfg_if::cfg_if;

use crate::Builder;
use crate::BuilderId;
use crate::Can;
use crate::CanBuilder;
use crate::CanSized;



/// Generalized artifact promise of a builder.
///
/// Implemented by `Blueprint` and `BlueprintUnsized`.
///
// typical bound: `where BCan: Can<B>`
pub trait Promise<B: ?Sized, BCan>: Debug + 'static {
	/// Get the unique id of the inner builder.
	///
	fn id(&self) -> BuilderId;

	/// Access the inner builder.
	///
	/// Notice: this function deliberately returns an opaque type with no
	/// methods, as a Promise is supposed to be opaque, but this
	/// accessor is required for this library to work.
	///
	fn builder(&self) -> BuilderAccessor<B>;

	/// Get the inner builder in a opaque can.
	///
	/// Notice: this function deliberately returns an opaque type with no
	/// methods, as a Promise is supposed to be opaque, but this
	/// accessor is required for this library to work.
	///
	fn canned(&self) -> CannedAccessor<BCan>;
}


/// Opaque builder accessor, used internally.
pub struct BuilderAccessor<'a, B: ?Sized> {
	pub(crate) builder: &'a B,
}


/// Opaque canned builder accessor, used internally.
pub struct CannedAccessor<BCan> {
	pub(crate) can: BCan,
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
pub struct Blueprint<B, BCan: Can<B>> {
	builder: BCan::Bin,
	_dummy: (),
}

impl<B, BCan: CanSized<B>> Blueprint<B, BCan> {
	/// Crates a new promise for the given builder.
	///
	pub fn new(builder: B) -> Self {
		let bin = BCan::into_bin(builder);

		Self::new_binned(bin)
	}
}

impl<B, BCan: Can<B>> Blueprint<B, BCan> {
	/// Create a new promise for the given binned builder.
	///
	pub(crate) fn new_binned(builder_bin: BCan::Bin) -> Self {
		Blueprint {
			builder: builder_bin,
			_dummy: (),
		}
	}

	/// Returns the pointer to the inner builder.
	///
	/// The returned pointer has a unspecific validity, thus it may only be used
	/// for comparing with other pointers but dereferencing it can never be
	/// considered safe.
	///
	pub(crate) fn builder_ptr(&self) -> *const () {
		BCan::bin_as_ptr(&self.builder) as *const ()
	}
}


impl<B, BCan: Can<B>> Blueprint<B, BCan> {
	/// Returns the id of this artifact promise
	/// This Id has the following property:
	/// The ids of two artifact promises are the same if and only if
	/// they point to the same builder.
	pub fn id(&self) -> BuilderId {
		BuilderId::new(BCan::bin_as_ptr(&self.builder))
	}
}

impl<B, BCan: CanSized<B>> Promise<B, BCan> for Blueprint<B, BCan>
		where
			B: 'static,
			BCan::Bin: AsRef<B> + Clone, {

	fn id(&self) -> BuilderId {
		self.id()
	}

	fn builder(&self) -> BuilderAccessor<B> {
		BuilderAccessor {
			builder: self.builder.as_ref(),
		}
	}

	fn canned(&self) -> CannedAccessor<BCan> {
		CannedAccessor {
			can: BCan::from_bin(self.builder.clone()),
		}
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		use crate::CanUnsized;

		impl<B, BCan> Blueprint<B, BCan> where
				BCan: CanSized<B>,
				BCan::Bin: Clone, {

			/// Makes this sized artifact promise into an unsized artifact
			/// promise.
			///
			/// **Notice: This function is only available if the `unsized`
			/// feature has been activated**.
			///
			/// An unsized artifact promise might represent for instance
			/// a builder by trait object. This allows in some cases to support
			/// multiple builders without adding additional type parameters.
			///
			pub fn into_unsized<UB: ?Sized + 'static>(self) -> BlueprintUnsized<UB, BCan>
				where
					B: 'static + std::marker::Unsize<UB>,
					BCan: CanUnsized<B, UB> {

				BlueprintUnsized::new_binned(self.builder).into_unsized()
			}
		}
	}
}

impl<B, BCan: Can<B>> Clone for Blueprint<B, BCan> where BCan::Bin: Clone {
	fn clone(&self) -> Self {
		Blueprint {
			builder: self.builder.clone(),
			_dummy: (),
		}
	}
}

impl<B, BCan: Can<B>> Hash for Blueprint<B, BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl<B, BCan: Can<B>> PartialEq for Blueprint<B, BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id().eq(&other.id())
	}
}

impl<B, BCan: Can<B>> Eq for Blueprint<B, BCan> {
}

impl<B, BCan: Can<B>> fmt::Pointer for Blueprint<B, BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", BCan::bin_as_ptr(&self.builder))
	}
}

impl<B, BCan: Can<B>> fmt::Debug for Blueprint<B, BCan> where BCan::Bin: fmt::Debug {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "ArtifactPromise {{builder: {:?}, id: {:p}}}", self.builder, self.id())
	}
}

impl<B, BCan: CanSized<B>> From<B> for Blueprint<B, BCan> where BCan::Bin: fmt::Debug {
	fn from(builder: B) -> Self {
		Blueprint::new(builder)
	}
}


/// Encapsulates a `Builder` as promise for its artifact from the `ArtifactCache`.
///
/// This struct is essentially a wrapper around two `Rc`s, but it provides a
/// `Hash` and `Eq` implementation based on the identity of the `Rc`s inner
/// value. In other words the address of the allocation behind the Rc is
/// compared instead of the semantics (also see [`Rc::ptr_eq()`]).
/// Thus all clones of an `ArtifactPromise` are considered identical.
///
/// Also see `ArtifactPromise`, which only requires a single `Rc`. Therefore
/// when ever possible `ArtifactPromise` should be preferred over this type.
/// This type exists only to allow for unsized builders that is a builder as
/// trait object (aka `dyn Builder`).
///
/// An `ArtifactPromise` can be either resolved using the `ArtifactCache::get()`
/// or `ArtifactResolver::resolve()`, whatever is available.
///
/// [`Rc::ptr_eq()`]: https://doc.rust-lang.org/std/rc/struct.Rc.html#method.ptr_eq
///
pub struct BlueprintUnsized<B: ?Sized, BCan: Can<B>> {
	builder: BCan::Bin,
	builder_canned: BCan,
	_dummy: (),
}

impl<B, BCan: CanSized<B>> BlueprintUnsized<B, BCan> where BCan::Bin: Clone {
	/// Crates a new promise for the given builder.
	///
	pub fn new(builder: B) -> Self
			where
				BCan: CanSized<B>, {

		let bin = BCan::into_bin(builder);

		Self::new_binned(bin)
	}
}

impl<B, BCan: CanSized<B>> BlueprintUnsized<B, BCan> where BCan::Bin: Clone {
	/// Create a new promise for the given binned builder.
	///
	pub(crate) fn new_binned(builder_bin: BCan::Bin) -> Self {
		BlueprintUnsized {
			builder: builder_bin.clone(),
			builder_canned: BCan::from_bin(builder_bin),
			_dummy: (),
		}
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		impl<B: ?Sized, BCan> BlueprintUnsized<B, BCan> where
				BCan: Can<B>, {

			/// Converts this artifact promise from type `B` to `UB` via
			/// unsizing.
			///
			/// **Notice: This function is only available if the `unsized`
			/// feature has been activated**.
			///
			/// Unsizing is typically involved when using trait objects. Thus if
			/// artifact promise of a `dyn Builder` is needed this function can
			/// be used to convert an artifact promise of a specific builder to
			/// an artifact promise of a trait object, if compatible.
			///
			pub fn into_unsized<UB: ?Sized + 'static>(self) -> BlueprintUnsized<UB, BCan>
				where
					B: 'static + std::marker::Unsize<UB>,
					BCan: CanUnsized<B, UB> {

				//let b: Rc<UB> = self.builder;

				BlueprintUnsized {
					builder: BCan::into_unsized(self.builder),
					builder_canned: self.builder_canned,
					_dummy: (),
				}
			}
		}
	}
}

impl<B: ?Sized, BCan: Can<B>> BlueprintUnsized<B, BCan> {
	/// Returns the id of this artifact promise
	/// This Id has the following property:
	/// The ids of two artifact promises are the same if and only if
	/// they point to the same builder.
	pub fn id(&self) -> BuilderId {
		BuilderId::new(BCan::can_as_ptr(&self.builder_canned))
	}

	/// Returns the pointer to the inner builder.
	///
	/// The returned pointer has a unspecific validity, thus it may only be used
	/// for comparing with other pointers but dereferencing it can never be
	/// considered safe.
	///
	pub(crate) fn builder_ptr(&self) -> *const () {
		BCan::can_as_ptr(&self.builder_canned) as *const ()
	}

	/// Constructs a truly unsized instance from two clones of the same builder.
	///
	/// # Panic
	///
	/// Panics if the two arguments are not the same `Rc`.
	///
	/// # Deprecated
	///
	/// This function can not absorb the builder itself so there might remain
	/// e.g. `Rc`-clones keeping the inner builder accessible form the outside.
	/// This avoids the effects of this type which is opaque encapsulation.
	/// Therefore this function might be removed in the future when an
	/// alternative stable way to construct a truly unsized promise has been
	/// found.
	///
	// TODO: This is currently the only way to instantiate a truly unsized AP,
	// however, it breaks encapsulation, as the inner Builder might be still
	// accessible from the outside. Therefore, it should be removed as soon as
	// `unsize` becomes stable.
	#[deprecated = "breaks encapsulation, will be removed"]
	pub fn from_clones(builder_bin: BCan::Bin, builder_can: BCan) -> Option<Self> {
		if BCan::bin_as_ptr(&builder_bin) == BCan::can_as_ptr(&builder_can) as *const () {
			Some(
				BlueprintUnsized {
					builder: builder_bin,
					builder_canned: builder_can,
					_dummy: (),
				}
			)
		} else {
			None
		}
	}
}

impl<ArtCan, BCan, Artifact, DynState> BlueprintUnsized<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState>, BCan> where
	BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState>> {

	/// Creates an trait object artifact promise from given builder.
	///
	pub fn new_unsized<B>(builder: B) -> Self
		where
			BCan: CanBuilder<ArtCan, Artifact, DynState, B>, {

		let (bin_dyn, can) = BCan::can_unsized(builder);

		BlueprintUnsized {
			builder: bin_dyn,
			builder_canned: can,
			_dummy: (),
		}
	}
}


impl<B: ?Sized, BCan: Can<B>> Promise<B, BCan> for BlueprintUnsized<B, BCan>
		where
			B: 'static,
			BCan::Bin: AsRef<B>,
			BCan: Clone, {

	fn id(&self) -> BuilderId {
		self.id()
	}

	fn builder(&self) -> BuilderAccessor<B> {
		BuilderAccessor {
			builder: self.builder.as_ref(),
		}
	}

	fn canned(&self) -> CannedAccessor<BCan> {
		CannedAccessor {
			can: self.builder_canned.clone(),
		}
	}
}

impl<B: ?Sized, BCan: Can<B>> Clone for BlueprintUnsized<B, BCan> where BCan::Bin: Clone, BCan: Clone {
	fn clone(&self) -> Self {
		BlueprintUnsized {
			builder: self.builder.clone(),
			builder_canned: self.builder_canned.clone(),
			_dummy: (),
		}
	}
}



impl<B: ?Sized, BCan: Can<B>> Hash for BlueprintUnsized<B, BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl<B: ?Sized, BCan: Can<B>> PartialEq for BlueprintUnsized<B, BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id().eq(&other.id())
	}
}

impl<B: ?Sized, BCan: Can<B>> Eq for BlueprintUnsized<B, BCan> {
}

impl<B: ?Sized, BCan: Can<B>> fmt::Pointer for BlueprintUnsized<B, BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", BCan::can_as_ptr(&self.builder_canned))
	}
}

impl<B: ?Sized, BCan: Can<B>> fmt::Debug for BlueprintUnsized<B, BCan> where BCan::Bin: fmt::Debug {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "BlueprintUnsized {{builder: {:?}, id: {:p}}}", self.builder, self.id())
	}
}

impl<B, BCan: CanSized<B>> From<B> for BlueprintUnsized<B, BCan> where BCan::Bin: fmt::Debug + Clone {
	fn from(builder: B) -> Self {
		BlueprintUnsized::new(builder)
	}
}


