
//!
//! Module for canning values.
//!
//! This module essentially fulfils the need of this crate to store various
//! types of Artifacts and Builders within the same structure. This is
//! essentially achieved by casting to some `dyn Any` (which is why Builders and
//! Artifacts have to be `'static`). Since `dyn Any` is unsized, it cannot be
//! stored per-se and requires some container such as `Rc`, `Arc`, or `Box`.
//!
//! Originally `daab` had simply used `Rc`s. But there is no good reason for
//! not using any of the other container types. Thus the current `daab`
//! implementation is generic over this container type, and this canning
//! infrastructure traits have been conceived to to allow the required
//! operations (most importantly downcasting) on them while keeping it as open
//! and generic as reasonable.
//!
//! The afore mentioned container types for some `dyn Any` is here referred to
//! as _Can_. Because the `dyn Any` type is opaque but uniform, allowing the
//! store various value of different internal type using the same Can-type.
//! Other words various `T`-values may be casted to same Can-type such as
//! `Rc<dyn Any>`.
//!
//! In the above example, the `T`-value has to wrapped in an `Rc<T>` first. This
//! `Rc<T>` as opposed to the Can, is specific and transparent and will be
//! following referred to as _Bin_-type of a Can-type for some `T`.
//!
//! This completes the basics this module is based on. Can (such as
//! `Rc<dyn Any>`) are supposed to implement the [`Can`] trait, which is
//! generic `T` for all the values which may be contained (e.g. restricted to
//! `T: 'static`). `Can` then has the associated type [`Bin`], which defines
//! for the implementing Can what the transparent wrapper for the specific `T`
//! is, for instance the Can `Rc<dyn Any>` simply defines the Bin as `Rc<T>`
//! for each `T`.
//!
//! The further traits simply define various properties which may or may not be
//! implemented for a specific Can and `T`. For instance the `CanRefMut` trait
//! is only implemented for `Box<dyn Any>` as it is the only one (of the std
//! types) which allows mutable access to the wrapped value. While `CanSized`
//! can only be implemented for `T: Sized` (as opposed to `T: ?Sized`).
//!
//! Notice in addition to the implementations for `Rc`, `Arc`, `Box` and
//! `BuilderArtifact`, which are provided as part of this crate, it is possible
//! to implement the various Can-Trait for any custom container type.
//!
//! [`Can`]: trait.Can.html
//! [`Bin`]: trait.Can.html#associatedtype.Bin
//!

use std::ops::Deref;
use std::fmt::Debug;
use std::any::Any;

use cfg_if::cfg_if;

use crate::Builder;

cfg_if! {
	if #[cfg(feature = "unsized")] {
		use std::marker::Unsize;
	}
}



/// Represents an opaque wrapper for `dyn Any`.
///
/// This trait represents an opaque wrapper for some `dyn Any`. It is basis for
/// the [`Can`] trait which is further implemented for various `T` specifying a
/// respective [`Bin`] type.
///
/// A `CanBase` is implemented for instance by `Rc<dyn Any>`, `Arc<dyn Any>`, or
/// `Box<dyn Any>`.
///
/// Since a Can is typically a smart-pointer as the examples above are, Cans &
/// Bins (as defined by the sub-trait [`Can`]) are supposed to produce a pointer
/// to the inner value (e.g. via [`can_as_ptr`] or [`bin_as_ptr`]), which has to
/// be the same regardless of whether it is retrieved from the `Can` or `Bin`.
///
/// Also see [`Can`].
///
/// [`Can`]: trait.Can.html
/// [`Bin`]: trait.Can.html#associatedtype.Bin
/// [`can_as_ptr`]: trait.CanBase.html#tymethod.can_as_ptr
/// [`bin_as_ptr`]: trait.Can.html#tymethod.bin_as_ptr
///
// Impl for Rc, Arc, Box, Bp
pub trait CanBase: Debug + Sized + 'static {
	/// Returns the pointer to the inner value.
	///
	fn can_as_ptr(&self) -> *const dyn Any;
}

/// Represents an opaque wrapper for `dyn Any` which has a transparent
/// representation for `T`.
///
/// A `CanBase` is for instance a `Rc<dyn Any>`, `Arc<dyn Any>`, or
/// `Box<dyn Any>`, which has transparent representation (a [`Bin`]) for any `T`
/// as `Rc<T>`, `Arc<T>`, or `Box<T>`, respectively.
///
/// The `Can` is the essential basis trait, which defines the `Bin` for its `T`.
/// This `Bin` than is used by various sub-traits to define specific methods
/// and functions on it. For instance the [`CanSized`] trait defines conversion
/// functions from `T` to `Bin` to `Can` and with its [`downcast_can`] method an
/// important way-back from a `Can` to a `Bin`.
///
/// Since a Can is typically a smart-pointer as the examples above are, Cans &
/// Bins are supposed to produce a pointer
/// to the inner value (e.g. via [`can_as_ptr`] or [`bin_as_ptr`]), which has to
/// be the same (i.e. same numeric value) regardless of whether it is retrieved
/// from the `Can` or `Bin`.
///
/// [`Bin`]: trait.Can.html#associatedtype.Bin
/// [`CanSized`]: trait.CanSized.html
/// [`downcast_can`]: trait.CanSized.html#tymethod.downcast_can
/// [`can_as_ptr`]: trait.CanBase.html#tymethod.can_as_ptr
/// [`bin_as_ptr`]: trait.Can.html#tymethod.bin_as_ptr
///
// Impl for Rc, Arc, Box, Bp for <T: ?Sized>
pub trait Can<T: ?Sized>: CanBase {
	/// A specific transparent wrapper for `T` convertible to and from `Self`.
	///
	/// For instance `Rc<dyn Any>`, which implements this trait, defines this
	/// `Bin` for any `T` as `Rc<T>`. That `Rc<T>` can easily coerced to
	/// `Rc<dyn Any>`, and casted back to `Rc<T>`.
	///
	type Bin: Debug + 'static;

	/// Returns the pointer to inner value.
	///
	fn bin_as_ptr(b: &Self::Bin) -> *const ();
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		/// Can allowing unsized conversion.
		///
		/// **Notice: This trait is only available if the `unsized`
		/// feature has been activated**.
		///
		#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "unsized")))]
		pub trait CanUnsized<T: ?Sized, UT: ?Sized>: Can<T> + Can<UT> {

			/// Convert the inner type in accordance with unsized.
			///
			fn into_unsized(bin: <Self as Can<T>>::Bin) -> <Self as Can<UT>>::Bin;
		}
	}
}

/// Sized variant of `Can`.
///
// Impl for Rc, Arc, Box, Bp for <T: Sized>
pub trait CanSized<T>: Can<T> {
	/// Create a `Bin` from `T`.
	///
	fn into_bin(t: T) -> Self::Bin;

	/// Create `Self` directly from `T`.
	///
	fn from_inner(t: T) -> Self {
		Self::from_bin(Self::into_bin(t))
	}

	/// Creates Self form a `Bin`.
	///
	/// This is a upcast and may not fail, as opposed to [`downcast_can`].
	///
	/// [`downcast_can`]: trait.CanSized.html#tymethod.downcast_can
	///
	// NOTICE this function might not require T: Sized, but as of now casting
	// (up & down) requires it in the implementation anyway
	fn from_bin(b: Self::Bin) -> Self;

	/// Tries to downcast the opaque `Can` to an specific `Bin`.
	///
	/// Because `Can`s are supposed to contain `dyn Any` allowing various `T`s
	/// to be casted to the same `Can`, this operation inherently may fail if
	/// the wrong `T` has been chosen, thus returning `None` is these cases.
	///
	/// However the following is supposed to work for any `CanType` (if it
	/// implements `CanSized`):
	/// ```
	/// # use std::rc::Rc;
	/// # use std::any::Any;
	/// # type CanType = Rc<dyn Any>;
	/// use daab::canning::CanSized;
	/// #[derive(Debug)]
	/// struct Foo;
	///
	/// // Up cast some `Foo`, and then downcast it back
	/// let can = <CanType as CanSized<Foo>>::from_inner(Foo);
	/// let bin = <CanType as CanSized<Foo>>::downcast_can(can);
	/// // This is supposed to work, because the can does contain a `Foo`
	/// assert!(bin.is_some());
	/// ```
	///
	// NOTICE this function might not require T: Sized, but as of now casting
	// (up & down) requires it in the implementation anyway
	fn downcast_can(self) -> Option<Self::Bin>;
}

/// Can that has a weak representation.
///
/// In the context of reference counting, a weak representation is supposed to
/// only allow access if there is at least one strong representation left.
/// This is a good representation for caching, since it can be used to
/// determine whether there is any active user left (who has to have a strong
/// representation).
///
/// For instance `Rc<dyn Any>`, which implements `CanStrong`, defines
/// `std::rc::Weak` as its `CanWeak`.
///
// Impl for Rc, Arc
pub trait CanStrong: CanBase {
	/// The weak representation for this type.
	type CanWeak: Debug;

	/// Allows to obtain a weak value for this can type.
	fn downgrade(&self) -> Self::CanWeak;

	/// Tries to upgrade a weak to a strong value, if there was any other
	/// strong value left.
	fn upgrade_from_weak(weak: &Self::CanWeak) -> Option<Self>;
}

/// Can with reference access.
///
/// This trait allows to get `T` by reference out of the Can though
/// [`downcast_can_ref`]. The reference is feed from the Can itself, thus
/// eliminating any cloning as it is might be needed when using
/// [`downcast_can`]. Thus if reference access is sufficient and available,
/// `downcast_can_ref` should be preferred over `downcast_can`.
///
/// [`downcast_can_ref`]: trait.CanRef.html#tymethod.downcast_can_ref
/// [`downcast_can`]: trait.CanSized.html#tymethod.downcast_can
///
// NOTICE: Can<T> would be sufficient as trait bound, but in this crate,
// CanRef<T> is always used together with CanSized<T>, and this way, the latter
// trait bound can be omitted in several places.
//
// NOTICE this function might not require T: Sized, but as of know casting
// (up & down) requires it in the implementation anyway
//
// Impl for Rc, Arc, Box for <T: Sized>
pub trait CanRef<T>: CanSized<T> {

	/// Tries to downcast the opaque `Can` to a reference to inner value.
	///
	/// Because `Can`s are supposed to contain `dyn Any` allowing various `T`s
	/// to be casted to the same `Can`, this operation inherently may fail if
	/// the wrong `T` has been chosen, thus returning `None` is these cases.
	///
	/// This is analogue to [`downcast_can`]. In fact, if for a Can-type
	/// both methods are available, then for a specific `self` and
	/// `T` they should either both fail, or both work.
	///
	/// The following is supposed to work for any `CanType` (if it
	/// implements `CanRef`):
	/// ```
	/// # use std::rc::Rc;
	/// # use std::any::Any;
	/// # type CanType = Rc<dyn Any>;
	/// use daab::canning::CanSized;
	/// use daab::canning::CanRef;
	/// #[derive(Debug)]
	/// struct Foo;
	///
	/// // Up cast some `Foo`, and then downcast it back
	/// let can = <CanType as CanSized<Foo>>::from_inner(Foo);
	/// let bin = <CanType as CanRef<Foo>>::downcast_can_ref(&can);
	/// // This is supposed to work, because the can does contain a `Foo`
	/// assert!(bin.is_some());
	/// ```
	///
	/// [`downcast_can`]: trait.CanSized.html#tymethod.downcast_can
	///
	fn downcast_can_ref(&self) -> Option<&T>;

}

/// Can with mutable reference access.
///
/// This trait allows to get `T` by mutable reference out of the Can though
/// [`downcast_can_mut`]. The reference is feed from the Can itself, thus
/// eliminating any cloning as it is might be needed when using
/// [`downcast_can`]. This is the mutable pendant to [`CanRef`].
///
/// Notice this is a special trait that is not widely implemented (here it is
/// only implemented for `Box<dyn Any>`).
///
/// [`downcast_can_mut`]: trait.CanRefMut.html#tymethod.downcast_can_mut
/// [`downcast_can`]: trait.CanSized.html#tymethod.downcast_can
/// [`CanRef`]: trait.CanRef.html
///
// NOTICE: Can<T> would be sufficient as trait bound, but in this crate,
// CanRef<T> is always used together with CanSized<T>, and this way, the latter
// trait bound can be omitted in several places.
//
// NOTICE this function might not require T: Sized, but as of know casting
// (up & down) requires it in the implementation anyway
//
// Impl for Rc, Arc, Box for <T: Sized>
pub trait CanRefMut<T>: CanSized<T> {
	/// Tries to downcast the opaque `Can` to a reference to inner value.
	///
	/// Because `Can`s are supposed to contain `dyn Any` allowing various `T`s
	/// to be casted to the same `Can`, this operation inherently may fail if
	/// the wrong `T` has been chosen, thus returning `None` is these cases.
	///
	/// This is analogue to [`downcast_can`]. In fact, if for a Can-type
	/// both methods are available, then for a specific `self` and
	/// `T` they should either both fail, or both work.
	///
	/// The following is supposed to work for any `CanType` (if it
	/// implements `CanRef`):
	/// ```
	/// # use std::any::Any;
	/// # type CanType = Box<dyn Any>;
	/// use daab::canning::CanSized;
	/// use daab::canning::CanRefMut;
	/// #[derive(Debug)]
	/// struct Foo;
	///
	/// // Up cast some `Foo`, and then downcast it back
	/// let mut can = <CanType as CanSized<Foo>>::from_inner(Foo);
	/// let bin = <CanType as CanRefMut<Foo>>::downcast_can_mut(&mut can);
	/// // This is supposed to work, because the can does contain a `Foo`
	/// assert!(bin.is_some());
	/// ```
	///
	/// [`downcast_can`]: trait.CanSized.html#tymethod.downcast_can
	///
	fn downcast_can_mut(&mut self) -> Option<&mut T>;
}


/// Referes to the Bin type of given BCan, if BCan is a `CanBuilder`.
///
pub type DynBuilderBin<ArtCan, BCan, Artifact, DynState, Err> =
	<BCan as Can<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState, Err=Err>>>::Bin;

/// A Can that can hold and convert a given builder into a Can of `dyn Builder`.
///
/// This is a specialized trait used to create unsized Builder Cans with
/// Stable Rust, as opposed to a more general approach that requires
/// Nightly Rust.
///
/// See [`BlueprintUnsized::new_unsized`] for its usage.
///
/// [`BlueprintUnsized::new_unsized`]: blueprint/struct.BlueprintUnsized.html#method.new_unsized
///
pub trait CanBuilder<ArtCan, Artifact, DynState, Err, B>:
		CanStrong +
		Can<B> +
		Can<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err>>
	{

	/// Create a unsized bin from given builder.
	///
	fn can_unsized(builder: <Self as Can<B>>::Bin) -> (DynBuilderBin<ArtCan, Self, Artifact, DynState, Err>, Self);
}

/*
TODO this is yet total unused!!!

/// A Can that can hold and convert a given builder into a Can of
/// `dyn Builder`, `Sync` variant.
///
/// This is a specialized trait used to create unsized and `Sync` Builder Cans.
/// See [`BlueprintUnsized::new_unsized`] usage of it.
///
///
pub trait CanBuilderSync<ArtCan, Artifact, DynState, Err, B>:
		CanStrong +
		Can<B> +
		Can<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err> + Send + Sync>
	{

	/// Create a unsized bin from given builder.
	///
	fn can_unsized(builder: B) -> (<Self as Can<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err> + Send + Sync>>::Bin, Self);
}
*/


//
// Rc impls
//

use std::rc::Rc;
use std::rc::Weak as WeakRc;

impl CanBase for Rc<dyn Any> {
	fn can_as_ptr(&self) -> *const dyn Any {
		self.deref()
	}
}

impl CanStrong for Rc<dyn Any> {
	type CanWeak = WeakRc<dyn Any>;

	fn downgrade(&self) -> Self::CanWeak {
		Rc::downgrade(self)
	}

	fn upgrade_from_weak(weak: &Self::CanWeak) -> Option<Self> {
		weak.upgrade()
	}
}

impl<T: ?Sized + Debug + 'static> Can<T> for Rc<dyn Any> {
	type Bin = Rc<T>;

	fn bin_as_ptr(b: &Self::Bin) -> *const () {
		b.deref() as *const T as *const ()
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		impl<T, UT> CanUnsized<T, UT> for Rc<dyn Any>
				where
					T: ?Sized + Debug + 'static,
					UT: ?Sized + Debug + 'static,
					T: Unsize<UT> {

			fn into_unsized(bin: <Self as Can<T>>::Bin) -> <Self as Can<UT>>::Bin {
				/*
				let input: Rc<T> = bin;
				let output: Rc<UT> = input;
				output
				*/
				bin
			}
		}
	}
}

impl<T: Debug + 'static> CanRef<T> for Rc<dyn Any> {
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
}

impl<T: Debug + 'static> CanSized<T> for Rc<dyn Any> {
	fn into_bin(t: T) -> Self::Bin {
		Rc::new(t)
	}
	fn downcast_can(self) -> Option<Self::Bin> {
		self.downcast().ok()
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
}

impl<ArtCan: 'static, Artifact, DynState, Err, B> CanBuilder<ArtCan, Artifact, DynState, Err, B> for Rc<dyn Any>
	where
		B: Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err> + 'static,
		Artifact: Debug + 'static,
		DynState: Debug + 'static,
		Err: Debug + 'static,
		 {

	fn can_unsized(builder: Rc<B>) -> (
			DynBuilderBin<ArtCan, Self, Artifact, DynState, Err>, Self) {

		let rc = builder;

		let rc_dyn: Rc<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err>> =
			rc.clone();

		let rc_any: Rc<dyn Any> = rc;

		(
			rc_dyn,
			rc_any,
		)
	}
}



//
// Box impls
//

impl CanBase for Box<dyn Any> {
	fn can_as_ptr(&self) -> *const dyn Any {
		self.deref()
	}
}

impl<T: ?Sized + Debug + 'static> Can<T> for Box<dyn Any> {
	type Bin = Box<T>;

	fn bin_as_ptr(b: &Self::Bin) -> *const () {
		b.deref() as *const T as *const ()
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		impl<T, UT> CanUnsized<T, UT> for Box<dyn Any>
				where
					T: ?Sized + Debug + 'static,
					UT: ?Sized + Debug + 'static,
					T: Unsize<UT> {

			fn into_unsized(bin: <Self as Can<T>>::Bin) -> <Self as Can<UT>>::Bin {
				bin
			}
		}
	}
}

impl<T: Debug + 'static> CanRef<T> for Box<dyn Any> {
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
}

impl<T: Debug + 'static> CanRefMut<T> for Box<dyn Any> {
	fn downcast_can_mut(&mut self) -> Option<&mut T> {
		self.downcast_mut()
	}
}

impl<T: Debug + 'static> CanSized<T> for Box<dyn Any> {
	fn into_bin(t: T) -> Self::Bin {
		Box::new(t)
	}
	fn downcast_can(self) -> Option<Self::Bin> {
		self.downcast().ok()
		//	.map(|r: &T| Box::new(r.clone()))
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
}



//
// Arc impls
//

use std::sync::Arc;
use std::sync::Weak as WeakArc;

impl CanBase for Arc<dyn Any + Send + Sync> {
	fn can_as_ptr(&self) -> *const dyn Any {
		self.deref()
	}
}

impl CanStrong for Arc<dyn Any + Send + Sync> {
	type CanWeak = WeakArc<dyn Any + Send + Sync>;

	fn downgrade(&self) -> Self::CanWeak {
		Arc::downgrade(self)
	}

	fn upgrade_from_weak(weak: &Self::CanWeak) -> Option<Self> {
		weak.upgrade()
	}
}

impl<T: ?Sized + Debug + Send + Sync + 'static> Can<T> for Arc<dyn Any + Send + Sync> {
	type Bin = Arc<T>;

	fn bin_as_ptr(b: &Self::Bin) -> *const () {
		b.deref() as *const T as *const ()
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		impl<T, UT> CanUnsized<T, UT> for Arc<dyn Any + Send + Sync>
				where
					T: ?Sized + Debug + Send + Sync + 'static,
					UT: ?Sized + Debug + Send + Sync + 'static,
					T: Unsize<UT> {

			fn into_unsized(bin: <Self as Can<T>>::Bin) -> <Self as Can<UT>>::Bin {
				bin
			}
		}
	}
}

impl<T: Debug + Send + Sync + 'static> CanRef<T> for Arc<dyn Any + Send + Sync> {
	fn downcast_can_ref(&self) -> Option<&T> {
		self.downcast_ref()
	}
}

impl<T: Debug + Send + Sync + 'static> CanSized<T> for Arc<dyn Any + Send + Sync> {
	fn into_bin(t: T) -> Self::Bin {
		Arc::new(t)
	}
	fn downcast_can(self) -> Option<Self::Bin> {
		self.downcast().ok()
	}
	fn from_bin(b: Self::Bin) -> Self {
		b
	}
}

/*
impl<ArtCan: 'static, Artifact: 'static, DynState, Err, B> CanBuilderSync<ArtCan, Artifact, DynState, Err, B> for Arc<dyn Any + Send + Sync>
	where
		B: Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err> + Send + Sync + 'static,
		Artifact: Debug + Send + Sync + 'static,
		DynState: Debug + Send + Sync + 'static,
		Err: Debug + Send + Sync + 'static,
		 {

	fn can_unsized(builder: B) -> (
			<Self as Can<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err> + Send + Sync>>::Bin, Self) {

		let arc = Arc::new(builder);

		let arc_dyn: Arc<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err> + Send + Sync> =
			arc.clone();

		let arc_any: Arc<dyn Any + Send + Sync> = arc;

		(
			arc_dyn,
			arc_any,
		)
	}
}
*/



cfg_if! {
	if #[cfg(feature = "unsized")] {

		//
		// Blueprint impls
		//

		use crate::Blueprint as Bp;
		use crate::blueprint::BlueprintUnsized as Bpu;
		use crate::Promise;

		/// A Can-type that allows Builders produced as Artifacts.
		///
		/// This is a special container type that allows to can `Blueprints`.
		/// That enables them to be used as Artifact type of Builders.
		/// These Builders of Builders are also referred to as _Super Builders_.
		///
		#[derive(Debug, Clone)]
		pub struct BuilderArtifact<BCan>(BCan);

		impl<BCan: CanBase + 'static> CanBase for BuilderArtifact<BCan> {
			fn can_as_ptr(&self) -> *const dyn Any {
				self.0.can_as_ptr()
			}
		}

		impl<BCan: 'static, B: 'static> Can<Bp<B,BCan>> for BuilderArtifact<BCan>
				where BCan: Can<B> {

			type Bin = Bp<B, BCan>;

			fn bin_as_ptr(b: &Self::Bin) -> *const () {
				b.builder_ptr()
			}
		}

		impl<BCan: 'static, B: 'static> CanSized<Bp<B,BCan>> for BuilderArtifact<BCan>
				where BCan: CanSized<B> + Clone, BCan::Bin: AsRef<B> + Clone {

			fn into_bin(ap: Bp<B,BCan>) -> Self::Bin {
				ap
			}
			fn downcast_can(self) -> Option<Self::Bin> {
				self.0.downcast_can().map( |bin| {
					Bp::new_binned(bin)
				})
			}
			fn from_bin(b: Self::Bin) -> Self {
				BuilderArtifact(b.canned().can)
			}
		}




		impl<BCan: 'static, B: ?Sized + 'static> Can<Bpu<B,BCan>> for BuilderArtifact<BCan>
				where BCan: Can<B> {

			type Bin = Bpu<B, BCan>;

			fn bin_as_ptr(b: &Self::Bin) -> *const () {
				b.deref().builder_ptr()
			}
		}

		cfg_if! {
			if #[cfg(feature = "unsized")] {
				impl<BCan, B: ?Sized, UB: ?Sized> CanUnsized<Bpu<B,BCan>, Bpu<UB,BCan>> for BuilderArtifact<BCan>
						where
							BCan: CanUnsized<B, UB>,
							BCan: 'static,
							B: 'static,
							UB: 'static,
							B: Unsize<UB> {

					fn into_unsized(bin: <Self as Can<Bpu<B,BCan>>>::Bin) -> <Self as Can<Bpu<UB,BCan>>>::Bin {
						bin.into_unsized()
					}
				}
			}
		}

		impl<BCan: 'static, B: 'static> CanSized<Bpu<B,BCan>> for BuilderArtifact<BCan>
				where BCan: CanSized<B> + Clone, BCan::Bin: AsRef<B> + Clone {

			fn into_bin(ap: Bpu<B,BCan>) -> Self::Bin {
				ap
			}
			fn downcast_can(self) -> Option<Self::Bin> {
				self.0.downcast_can().map( |bin| {
					Bpu::new_binned(bin)
				})
			}
			fn from_bin(b: Self::Bin) -> Self {
				BuilderArtifact(b.canned().can)
			}
		}
	}
}

