//!
//! Blueprint wrapper for Builders.
//!
//! This module provides the opaque [`Blueprint`] and [`BlueprintUnsized`]
//! wrappers, which are used to wrap Builders for using with the [`Cache`].
//! This module also contains the [`Promise`] trait which is an abstraction
//! over `Blueprint` and `BlueprintUnsized` used as trait bound in the
//! `Cache`.
//!
//! The point of the `Blueprint` wrappers is that users can not interact with
//! a Builder once it has been wrap in a `Blueprint`, and only use it to
//! interact with the `Cache`. Thus enforcing encapsulation.
//!
//! `Blueprint`s typically use `Rc<dyn Any>` or `Arc<dyn Any>` as their
//! Builder-Can. Thus `Blueprint`s are Clone with the semantic of an `Rc` or
//! `Arc` respectively, i.e. you can clone `Blueprint` to get multiple owned
//! values which semantically refer to the same Builder. Thus if these clones
//! are used in turn in different Builders they will resolve it to the same
//! Artifact. Thus allowing to create DAG dependencies.
//!
//!
//! # Unsizedness
//!
//! An _unsized Builder_ refers here typically to a trait object Builder aka
//! a `dyn Builder`. This kind of unsized Builder has some special restrictions,
//! which are most important when trying to acquire a Blueprint referring to it.
//!
//! To enable conversion from a sized Blueprint to an unsized one, the `unsized`
//! feature is required, which in turn requires a Nightly Rust Compiler!
//! It enables the [`Blueprint::into_unsized`] and
//! [`BlueprintUnsized::into_unsized`] functions.
//!
//! To allow also Stable Rust to use unsized Builders, there exist the
//! [`BlueprintUnsized::new_unsized`] function, which is less general, but
//! works on Stable Rust and without additional features.
//!
//! Since it is sometimes reasonable to use unsized Builders, is supported
//! by the `BlueprintUnsized`, which in turn requires to store simultaneously a
//! Can and a Bin of that Builder instead of just a Bin as the `Blueprint`
//! dose. Thus the `Blueprint` should be preferred when using sized Builders.
//!
//! [`Cache`]: ../cache/struct.Cache.html
//! [`Promise`]: trait.Promise.html
//! [`Blueprint`]: struct.Blueprint.html
//! [`Blueprint::into_unsized`]: struct.Blueprint.html#method.into_unsized
//! [`BlueprintUnsized`]: struct.BlueprintUnsized.html
//! [`BlueprintUnsized::new_unsized`]: struct.BlueprintUnsized.html#method.new_unsized
//! [`BlueprintUnsized::into_unsized`]: struct.BlueprintUnsized.html#method.into_unsized
//!


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
use crate::Never;



/// Generalized Promise for a Artifact from a Builder.
///
/// This Promise can be honored at the [`Cache`].
///
/// This trait is a generalization over [`Blueprint`] and [`BlueprintUnsized`],
/// which are the two types implementing it.
///
/// [`Cache`]: ../cache/struct.Cache.html
/// [`Blueprint`]: struct.Blueprint.html
/// [`BlueprintUnsized`]: struct.BlueprintUnsized.html
///
pub trait Promise: Debug + 'static {
	type Builder: ?Sized + 'static + Debug;
	type BCan: Can<Self::Builder>;

	/// Get the unique id of the inner builder.
	///
	/// All clones of the same `Promise` have the same id, thus
	/// containing/sharing the same Builder and consequently will deliver the
	/// same Artifact form a `Cache`.
	///
	fn id(&self) -> BuilderId;

	/// Access the inner builder.
	///
	/// Notice: this function deliberately returns an opaque type with no
	/// methods, as a Promise is supposed to be opaque, but this
	/// accessor is required for this library to work.
	///
	fn builder(&self) -> BuilderAccessor<Self::Builder>;

	/// Get the inner builder in a opaque can.
	///
	/// Notice: this function deliberately returns an opaque type with no
	/// methods, as a Promise is supposed to be opaque, but this
	/// accessor is required for this library to work.
	///
	fn canned(&self) -> CannedAccessor<Self::BCan>;
}

/// Opaque builder accessor, used internally.
///
pub struct BuilderAccessor<'a, B: ?Sized> {
	pub(crate) builder: &'a B,
}


/// Opaque canned builder accessor, used internally.
///
pub struct CannedAccessor<BCan> {
	pub(crate) can: BCan,
}


/// Wraps a Builder as a blueprint for its artifact from the `Cache`.
///
/// This is a wrapper around the Bin of the Builder-Can containing the actual
/// Builder _(i.e. it contains `<BCan as Can<B>>::Bin`, e.g. a `Rc<B>` when
/// using the `rc` module)_. While it provides
/// access to the inner Builder for the [`Cache`], it is not accessible for
/// others. Thus enforcing that the Builder itself can not be accessed.
///
/// The `Blueprint` can be used as [`Promise`] to access the inner Builder's
/// Artifact and dynamic state through the [`Cache`].
///
/// [`Cache`]: ../cache/struct.Cache.html
/// [`Promise`]: trait.Promise.html
///
pub struct Blueprint<B, BCan: Can<B>> {
	builder: BCan::Bin,
}

impl<B, BCan: CanSized<B>> Blueprint<B, BCan> {
	/// Crates a new `Blueprint` for the given sized Builder.
	///
	pub fn new(builder: B) -> Self {
		let bin = BCan::into_bin(builder);

		Self::new_binned(bin)
	}
}

impl<B, BCan: Can<B>> Blueprint<B, BCan> {
	/// Create a new `Blueprint` for the given binned Builder.
	///
	/// Internal function only, it breaks encapsulation!
	///
	pub(crate) fn new_binned(builder_bin: BCan::Bin) -> Self {
		Blueprint {
			builder: builder_bin,
		}
	}

	/// Returns the pointer to the inner Builder.
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
	/// Returns the id of the inner Builder.
	///
	/// All clones of the same `Blueprint` have the same id, thus
	/// containing/sharing the same Builder and consequently will deliver the
	/// same Artifact form a `Cache`.
	///
	pub fn id(&self) -> BuilderId {
		BuilderId::new(BCan::bin_as_ptr(&self.builder))
	}
}

impl<B, BCan: CanSized<B>> Promise for Blueprint<B, BCan>
		where
			B: 'static + Debug,
			BCan::Bin: AsRef<B> + Clone, {

	type Builder = B;
	type BCan = BCan;

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

			/// Converts this `Blueprint` with a sized Builder into an
			/// `BlueprintUnsized` with an unsized Builder.
			///
			/// **Notice: This function is only available if the `unsized`
			/// feature has been activated**.
			///
			/// An unsized Builder might represent for instance
			/// a trait object Builder. This allows in some cases to support
			/// multiple different Builders without adding additional type
			/// parameters.
			///
			#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "unsized")))]
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
		write!(fmt, "Blueprint {{builder: {:?}, id: {:p}}}", self.builder, self.id())
	}
}

impl<B, BCan: CanSized<B>> From<B> for Blueprint<B, BCan> where BCan::Bin: fmt::Debug {
	fn from(builder: B) -> Self {
		Blueprint::new(builder)
	}
}


cfg_if! {
	if #[cfg(feature = "unsized")] {
		/// Wraps a Builder as a blueprint for its artifact from the `Cache` allowing
		/// unsized Builders.
		///
		/// This is a wrapper around the Bin of the Builder-Can and additionally the
		/// Can itself containing the actual Builder _(i.e. it contains
		/// `<BCan as Can<B>>::Bin` & `BCan`, e.g. a `Rc<B>` & `Rc<dyn Any>` when using
		/// the `rc` module)_. While it provides access to the inner Builder for the
		/// [`Cache`], it is not accessible for others. Thus enforcing that the Builder
		/// itself can not be accessed.
		///
		/// The `BlueprintUnsized` can be used as [`Promise`] to access the inner
		/// Builder's Artifact and dynamic state through the [`Cache`].
		///
		/// The `BlueprintUnsized` allows to use unsized Builders such as trait objects.
		///
		/// [`Cache`]: ../cache/struct.Cache.html
		/// [`Promise`]: trait.Promise.html
		///
		#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "unsized")))]
		pub struct BlueprintUnsized<B: ?Sized, BCan: Can<B>> {
			builder: BCan::Bin,
			builder_canned: BCan,
		}

		#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "unsized")))]
		impl<B, BCan: CanSized<B>> BlueprintUnsized<B, BCan> where BCan::Bin: Clone {
			/// Crates a new `BlueprintUnsized` for the given sized builder.
			///
			/// Notice since here the Builder is given by value, it may not be unsized!
			///
			/// Instead, either use [`new_unsized`] to create directly a blueprint with
			/// a trait object Builder. Or use this `new` and then use [`into_unsized`]
			/// to turn it into a blueprint with an unsized Builder. The latter method
			/// it more general but requires the `unsized` features which in turn
			/// requires a Nightly Rust Compiler.
			///
			/// [`new_unsized`]: struct.BlueprintUnsized.html#method.new_unsized
			/// [`into_unsized`]: struct.BlueprintUnsized.html#method.into_unsized
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
			/// Internal function only, it breaks encapsulation!
			///
			pub(crate) fn new_binned(builder_bin: BCan::Bin) -> Self {
				BlueprintUnsized {
					builder: builder_bin.clone(),
					builder_canned: BCan::from_bin(builder_bin),
				}
			}
		}

		impl<B: ?Sized, BCan> BlueprintUnsized<B, BCan> where
				BCan: Can<B>, {

			/// Converts the generic parameter of this `BlueprintUnsized` from
			/// type `B` to `UB` via unsizing.
			///
			/// **Notice: This function is only available if the `unsized`
			/// feature has been activated**.
			///
			/// An unsized Builder might represent for instance
			/// a trait object Builder. This allows in some cases to support
			/// multiple different Builders without adding additional type
			/// parameters.
			///
			#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "unsized")))]
			pub fn into_unsized<UB: ?Sized + 'static>(self) -> BlueprintUnsized<UB, BCan>
				where
					B: 'static + std::marker::Unsize<UB>,
					BCan: CanUnsized<B, UB> {

				BlueprintUnsized {
					builder: BCan::into_unsized(self.builder),
					builder_canned: self.builder_canned,
				}
			}
		}

		impl<B: ?Sized, BCan: Can<B>> BlueprintUnsized<B, BCan> {
			/// Returns the id of the inner Builder.
			///
			/// All clones of the same `Blueprint` have the same id, thus
			/// containing/sharing the same Builder and consequently will deliver the
			/// same Artifact form a `Cache`.
			///
			pub fn id(&self) -> BuilderId {
				BuilderId::new(BCan::can_as_ptr(&self.builder_canned))
			}

			/// Returns the pointer to the inner Builder.
			///
			/// The returned pointer has a unspecific validity, thus it may only be used
			/// for comparing with other pointers but dereferencing it can never be
			/// considered safe.
			///
			pub(crate) fn builder_ptr(&self) -> *const () {
				BCan::can_as_ptr(&self.builder_canned) as *const ()
			}
		}

		impl<ArtCan, BCan, Artifact, DynState, Err> BlueprintUnsized<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState, Err=Err>, BCan> where
			BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState, Err=Err>> {

			/// Creates an `BlueprintUnsized` with a trait object Builder with the given
			/// Builder.
			///
			/// An more general approach is to use [`new`] and then [`into_unsized`],
			/// but the latter requires the `unsized` features which in turn
			/// requires a Nightly Rust Compiler. Thus this `new_unsized` function
			/// is provided as means to generate a trait object Builder using only
			/// Stable Rust.
			///
			/// [`new`]: struct.BlueprintUnsized.html#method.new
			/// [`into_unsized`]: struct.BlueprintUnsized.html#method.into_unsized
			///
			pub fn new_unsized<B>(builder: B) -> Self
				where
					BCan: CanSized<B>,
					BCan: CanBuilder<ArtCan, Artifact, DynState, Err, B>, {

				let (bin_dyn, can) = BCan::can_unsized(BCan::into_bin(builder));

				BlueprintUnsized {
					builder: bin_dyn,
					builder_canned: can,
				}
			}

			/// Creates an `BlueprintUnsized` with a trait object Builder from the given
			/// 'sized' `BlueprintUnsized`.
			///
			pub fn from_sized<B>(blueprint: BlueprintUnsized<B,BCan>) -> Self
				where
					BCan: CanBuilder<ArtCan, Artifact, DynState, Err, B>, {

				let (bin_dyn, can) = BCan::can_unsized(blueprint.builder);

				BlueprintUnsized {
					builder: bin_dyn,
					builder_canned: can,
				}
			}

			/// Creates an `BlueprintUnsized` with a trait object Builder with the given
			/// 'sized' `Blueprint`.
			///
			pub fn from_sized_bp<B>(blueprint: Blueprint<B,BCan>) -> Self
				where
					BCan: CanBuilder<ArtCan, Artifact, DynState, Err, B>, {

				let (bin_dyn, can) = BCan::can_unsized(blueprint.builder);

				BlueprintUnsized {
					builder: bin_dyn,
					builder_canned: can,
				}
			}
		}


		impl<B: ?Sized, BCan: Can<B>> Promise for BlueprintUnsized<B, BCan>
				where
					B: Debug + 'static,
					BCan::Bin: AsRef<B>,
					BCan: Clone, {

			type Builder = B;
			type BCan = BCan;

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

		impl<B, BCan: CanSized<B>> From<Blueprint<B, BCan>> for BlueprintUnsized<B, BCan> where BCan::Bin: Clone {
			fn from(sized_bp: Blueprint<B, BCan>) -> Self {
				Self {
					builder: sized_bp.builder.clone(),
					builder_canned: BCan::from_bin(sized_bp.builder),
				}
			}
		}

		impl<B, BCan: CanSized<B>> From<B> for BlueprintUnsized<B, BCan> where BCan::Bin: fmt::Debug + Clone {
			fn from(builder: B) -> Self {
				BlueprintUnsized::new(builder)
			}
		}

		impl<B, ArtCan, BCan, Artifact, DynState, Err> From<Blueprint<B, BCan>> for BlueprintUnsized<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState, Err=Err>, BCan>
			where
				BCan: CanSized<B>,
				BCan: CanBuilder<ArtCan, Artifact, DynState, Err, B>,
				BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState, Err=Err>> {

			fn from(builder: Blueprint<B, BCan>) -> Self {
				BlueprintUnsized::from_sized_bp(builder)
			}
		}

		impl<B, ArtCan, BCan, Artifact, DynState, Err> From<BlueprintUnsized<B, BCan>> for BlueprintUnsized<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState, Err=Err>, BCan>
			where
				BCan: CanSized<B>,
				BCan: CanBuilder<ArtCan, Artifact, DynState, Err, B>,
				BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Artifact, DynState=DynState, Err=Err>> {

			fn from(builder: BlueprintUnsized<B, BCan>) -> Self {
				BlueprintUnsized::from_sized(builder)
			}
		}
	}
}




type BuilderDyn<ArtCan, BCan, Artifact, Err, DynState> = dyn Builder<ArtCan, BCan, Artifact=Artifact, Err=Err, DynState=DynState>;

/// Wraps a Builder as a blueprint for its artifact from the `Cache` allowing
/// unsized Builders.
///
/// This is a wrapper around the Bin of the Builder-Can and additionally the
/// Can itself containing the actual Builder _(i.e. it contains
/// `<BCan as Can<B>>::Bin` & `BCan`, e.g. a `Rc<B>` & `Rc<dyn Any>` when using
/// the `rc` module)_. While it provides access to the inner Builder for the
/// [`Cache`], it is not accessible for others. Thus enforcing that the Builder
/// itself can not be accessed.
///
/// The `BlueprintUnsized` can be used as [`Promise`] to access the inner
/// Builder's Artifact and dynamic state through the [`Cache`].
///
/// The `BlueprintUnsized` allows to use unsized Builders such as trait objects.
///
/// [`Cache`]: ../cache/struct.Cache.html
/// [`Promise`]: trait.Promise.html
///
pub struct BlueprintDyn<ArtCan, BCan, Art, Err=Never, DynSt=()>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>> {

	builder: BCan::Bin,
	builder_canned: BCan,
}

impl<ArtCan, BCan, Art, Err, DynSt> BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {

	/// Crates a new `BlueprintUnsized` for the given sized builder.
	///
	/// Notice since here the Builder is given by value, it may not be unsized!
	///
	/// Instead, either use [`new_unsized`] to create directly a blueprint with
	/// a trait object Builder. Or use this `new` and then use [`into_unsized`]
	/// to turn it into a blueprint with an unsized Builder. The latter method
	/// it more general but requires the `unsized` features which in turn
	/// requires a Nightly Rust Compiler.
	///
	/// [`new_unsized`]: struct.BlueprintUnsized.html#method.new_unsized
	/// [`into_unsized`]: struct.BlueprintUnsized.html#method.into_unsized
	///
	pub fn new<B>(builder: B) -> Self
			where
				Art: Debug + 'static,
				Err: Debug + 'static,
				DynSt: Debug + 'static,
				B: Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>,
				BCan: CanSized<B>,
				BCan: CanBuilder<ArtCan, Art, DynSt, Err, B>, {

		let (bin_dyn, can) = BCan::can_unsized(BCan::into_bin(builder));

		BlueprintDyn {
			builder: bin_dyn,
			builder_canned: can,
		}
	}
}

cfg_if! {
	if #[cfg(feature = "unsized")] {
		impl<ArtCan, BCan, Art, Err, DynSt> BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
			where
				BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {

			/// Converts the generic parameter of this `BlueprintUnsized` from
			/// type `B` to `UB` via unsizing.
			///
			/// **Notice: This function is only available if the `unsized`
			/// feature has been activated**.
			///
			/// An unsized Builder might represent for instance
			/// a trait object Builder. This allows in some cases to support
			/// multiple different Builders without adding additional type
			/// parameters.
			///
			#[cfg_attr(feature = "doc_cfg", doc(cfg(feature = "unsized")))]
			pub fn into_unsized(self) -> BlueprintUnsized<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>, BCan> {

				BlueprintUnsized {
					builder: self.builder,
					builder_canned: self.builder_canned,
				}
			}
		}
	}
}

impl<ArtCan, BCan, Art, Err, DynSt> BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {

	/// Returns the id of the inner Builder.
	///
	/// All clones of the same `Blueprint` have the same id, thus
	/// containing/sharing the same Builder and consequently will deliver the
	/// same Artifact form a `Cache`.
	///
	pub fn id(&self) -> BuilderId {
		BuilderId::new(BCan::can_as_ptr(&self.builder_canned))
	}

	/// Returns the pointer to the inner Builder.
	///
	/// The returned pointer has a unspecific validity, thus it may only be used
	/// for comparing with other pointers but dereferencing it can never be
	/// considered safe.
	///
	pub(crate) fn builder_ptr(&self) -> *const () {
		BCan::can_as_ptr(&self.builder_canned) as *const ()
	}
}

impl<ArtCan, BCan, Art, Err, DynSt> BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {

	/// Creates an `BlueprintUnsized` with a trait object Builder with the given
	/// 'sized' `Blueprint`.
	///
	pub fn from_bp<B>(blueprint: Blueprint<B,BCan>) -> Self
		where
			BCan: CanBuilder<ArtCan, Art, DynSt, Err, B>, {

		let (bin_dyn, can) = BCan::can_unsized(blueprint.builder);

		BlueprintDyn {
			builder: bin_dyn,
			builder_canned: can,
		}
	}
}


impl<ArtCan, BCan, Art, Err, DynSt> Promise for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		ArtCan: 'static,
		Art: 'static,
		Err: 'static,
		DynSt: 'static,
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>,
		BCan::Bin: AsRef<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>,
		BCan: Clone, {

	type Builder = dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>;
	type BCan = BCan;

	fn id(&self) -> BuilderId {
		self.id()
	}

	fn builder(&self) -> BuilderAccessor<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>> {
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

impl<ArtCan, BCan, Art, Err, DynSt> Clone for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>,
		BCan::Bin: Clone,
		BCan: Clone {

	fn clone(&self) -> Self {
		BlueprintDyn {
			builder: self.builder.clone(),
			builder_canned: self.builder_canned.clone(),
		}
	}
}



impl<ArtCan, BCan, Art, Err, DynSt> Hash for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {

	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id().hash(state);
	}
}

impl<ArtCan, BCan, Art, Err, DynSt> PartialEq for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {

	fn eq(&self, other: &Self) -> bool {
		self.id().eq(&other.id())
	}
}

impl<ArtCan, BCan, Art, Err, DynSt> Eq for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {
}

impl<ArtCan, BCan, Art, Err, DynSt> fmt::Pointer for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>, {

	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", BCan::can_as_ptr(&self.builder_canned))
	}
}

impl<ArtCan, BCan, Art, Err, DynSt> fmt::Debug for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>,
		BCan::Bin: fmt::Debug {

	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "BlueprintUnsized {{builder: {:?}, id: {:p}}}", self.builder, self.id())
	}
}

impl<ArtCan, BCan, Art, Err, DynSt, B> From<Blueprint<B, BCan>> for BlueprintDyn<ArtCan, BCan, Art, Err, DynSt>
	where
		//BCan: Can<dyn Builder<ArtCan, BCan, Artifact=Art, Err=Err, DynState=DynSt>>,
		//BCan: CanSized<B>,
		BCan: CanBuilder<ArtCan, Art, DynSt, Err, B>, {

	fn from(sized_bp: Blueprint<B, BCan>) -> Self {
		Self::from_bp(sized_bp)
	}
}

