
//!
//! Module for canning values.
//!
//! Canning means wrapping values in some package type, which is better for
//! storing. Thus these Can types contain some `dyn Any` value to allow casting
//! various values into cans.
//! In order to keep them more usable, a Can can be downcasted back to some `T`.
//!
//! This module also contains a notion for Bins which are 'open' Cans. For
//! instance an `Rc<dyn Any>` as one kind of Can, and its respective Bin is
//! `Rc<T>` for every `T`.
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
/// This type reperesents a wrapper for a `dyn Any`. It is basis for the `Can`
/// type which allows to be downcasted.
///
/// See `Can`.
///
// Impl for Rc, Arc, Box, Bp
pub trait CanBase: Debug + Sized + 'static {
	/// Returns the pointer to the inner value.
	///
	fn can_as_ptr(&self) -> *const dyn Any;
}

/// Represents an opaque wrapper for `dyn Any` which can be casted to `T`.
///
/// Since `dyn Any` can't be stored, a `Can` encapsules a `dyn Any` while
/// allowing it to be casted to specific wrapper `Bin` for `T`.
///
/// A good example for a `Can` is `Rc<dyn Any>`. Which for `T` can be casted
/// to a `Rc<T>` which would be the `Bin` type.
///
// Impl for Rc, Arc, Box, Bp for <T: ?Sized>
pub trait Can<T: ?Sized>: CanBase {
	/// A specific wrapper for `T` which can be casted from `Self`.
	///
	type Bin: Debug + 'static;

	/// Gets the pointer to
	fn bin_as_ptr(b: &Self::Bin) -> *const ();
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		/// Can allowing unsized conversion.
		///
		/// **Notice: This trait is only available if the `unsized`
		/// feature has been activated**.
		///
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
	/// Create a `Bin` for `T`.
	///
	fn into_bin(t: T) -> Self::Bin;

	/// Create `Self` directly from `T`.
	fn from_inner(t: T) -> Self {
		Self::from_bin(Self::into_bin(t))
	}

	/// Creates Self form a `Bin`.
	///
	/// This is a upcast and can not fail.
	///
	// NOTICE this function might not require T: Sized, but as of know casting
	// (up & down) requires it in the implementation anyway
	fn from_bin(b: Self::Bin) -> Self;

	/// Tries to downcast the opaque `Can` to an specific `Bin`.
	///
	/// Because `Can`s are supposed to be alike `Any` allowing various `T`s to
	/// be casted to the same `Can`, this operation inherently may fail.
	///
	// NOTICE this function might not require T: Sized, but as of know casting
	// (up & down) requires it in the implementation anyway
	fn downcast_can(self) -> Option<Self::Bin>;
}

/// Can that has a weak representation.
///
/// In the context of reference counting, a weak representation is supposed to
/// only allow access if there is at least one strong representation. It is a
/// good representation for caching, since it can be used to determine whether
/// there is any active user left (which has to have a strong representation).
///
/// Again the `Rc` type is a good example here, it is the `CanStrong` here and
/// the `std::rc::Weak` is the `CanWeak` in this regards.
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

/// Transparent variant of `Can`.
///
/// It allows additional to `Can` to get `T` from `Bin` and directly downcasting
/// this `Can` to `T`.
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

	/// Tries to downcast the opaque `Can` to an specific `T`, by passing the
	/// `Bin` and cloning.
	///
	fn downcast_can_ref(&self) -> Option<&T>;

}

/// Mutable transparent variant of `Can`.
///
/// It allows additional to `Can` to get `T` from `Bin` and directly downcasting
/// this `Can` to `T`.
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
	/// Tries to downcast the opaque `Can` to an specific `T`, by passing the
	/// `Bin` and cloning.
	///
	fn downcast_can_mut(&mut self) -> Option<&mut T>;
}

/// A can that can hold and convert a given builder into a can of `dyn Builder`
///
pub trait CanBuilder<ArtCan, Artifact, DynState, Err, B>:
		CanStrong +
		Can<B> +
		Can<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err>>
	{

	/// Create a unsized bin from given builder.
	///
	fn can_unsized(builder: B) -> (<Self as Can<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err>>>::Bin, Self);
}

/// A can that can hold and convert a given builder into a can of `dyn Builder`
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

impl<ArtCan: 'static, Artifact: 'static, DynState, Err, B> CanBuilder<ArtCan, Artifact, DynState, Err, B> for Rc<dyn Any>
	where
		B: Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err> + 'static,
		Artifact: Debug + 'static,
		DynState: Debug + 'static,
		Err: Debug + 'static,
		 {

	fn can_unsized(builder: B) -> (
			<Self as Can<dyn Builder<ArtCan, Self, Artifact=Artifact, DynState=DynState, Err=Err>>>::Bin, Self) {

		let rc = Rc::new(builder);

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



//
// Blueprint impls
//

use crate::Blueprint as Bp;
use crate::BlueprintUnsized as Bpu;
use crate::Promise;

/// A can type that can be used as an artifact can for builders.
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

