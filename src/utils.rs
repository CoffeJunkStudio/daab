//!
//! Utility module
//!
//! This module contains some utilities which can be useful when working with `daab`.
//!

use crate::Resolver;
use crate::Promise;
use crate::Blueprint;
use crate::Builder;
use crate::CanRef;
use crate::CanStrong;
use crate::CanSized;
use crate::Never;

use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;



/// A intermediate cached Builder to circumvent failing builders.
///
/// In resource loading, a resource might be come unavailable for sometime.
/// This wrapper builder will cache the result of its inner builder and return
/// the cached value instead, if the inner builder failed to produce a new
/// artifact.
///
/// **Notice: It is likely a logical error to keep an artifact despite
/// been invalidate, which is what this wrapper dose!**
///
/// Therefore, this wrapper should be used with care and only
/// with builders, which actually do not have any dependencies (so they might
/// only been manually invalidate) or having artifacts which are still usable
/// after any (or all) dependency has been invalidated.
///
/// # Panics
///
/// This builder panics in its `build` method if the first build of its inner
/// builder failed and the `default_value` has been set to `None`.
///
#[derive(Debug, Clone)]
pub struct RedeemingBuilder<AP, B: ?Sized, T> {
	inner: AP,
	default_value: Option<T>,
	_b: PhantomData<B>,
}

impl<AP, B: ?Sized, T> RedeemingBuilder<AP, B, T>
	where
		T: Clone,
		B: Debug + 'static, {

	/// Wrap given Builder and fill missing recreations with a previous value.
	///
	/// **Use with care**
	///
	pub fn new<ArtCan, BCan>(
		inner: AP,
		default_value: Option<T>
	) -> Blueprint<Self, BCan>
		where
			B: Builder<ArtCan, BCan, Artifact=Option<T>>,
			AP: Promise<B, BCan>,
			T: Clone + Debug + 'static,
			ArtCan: CanSized<Option<T>> + CanRef<Option<T>>,
			ArtCan::Bin: AsRef<Option<T>>,
			BCan: Clone + CanStrong,
			BCan: CanSized<Self>,
	{

		Blueprint::new(
			RedeemingBuilder {
				inner,
				default_value,
				_b: PhantomData,
			}
		)
	}
}

impl<ArtCan, AP, B: ?Sized, BCan, T> Builder<ArtCan, BCan> for RedeemingBuilder<AP, B, T>
	where
		B: Builder<ArtCan, BCan, Artifact=T>,
		AP: Promise<B, BCan>,
		T: Clone + Debug + 'static,
		ArtCan: CanSized<T> + CanRef<T>,
		ArtCan::Bin: AsRef<T>,
		BCan: Clone + CanStrong,
	{

	type Artifact = T;
	type DynState = Option<T>;
	type Err = Never;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			-> Result<Self::Artifact, Never> {

		let value = resolver.resolve_cloned(&self.inner);

		if let Ok(v) = value {
			*resolver.my_state() = Some(v.clone());

			// Return value
			Ok(v)

		} else {
			// Try to return cached value. Panics if very first build fails and
			// no default value was provided.
			// This is documented behavior.
			Ok(resolver.my_state().clone().unwrap())
		}
	}

	fn init_dyn_state(&self) -> Self::DynState {
		self.default_value.clone()
	}
}




/// Functional builder wrapper.
///
/// A functional builder is a builder consisting of a single function
/// `Fn(&mut Resolver) -> T`. Thus this type can be used to wrap a
/// closure as `Builder`. The return type `T` will the artifact type of the
/// resulting Builder.
///
pub struct FunctionalBuilder<ArtCan, BCan, F> {
	inner: F,
	_art_can: PhantomData<ArtCan>,
	_b_can: PhantomData<BCan>,
}

impl<ArtCan, BCan, F> Debug for FunctionalBuilder<ArtCan, BCan, F> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "FunctionalBuilder{{...}}")
	}
}

impl<ArtCan, BCan, F, T> FunctionalBuilder<ArtCan, BCan, F>
	where
		F: (for<'r, 's> Fn(&'r mut Resolver<'s, ArtCan, BCan>) -> T) + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		ArtCan: 'static {

	/// Wraps the given closure as Builder.
	///
	pub fn new(f: F) -> Self {
		FunctionalBuilder {
			inner: f,
			_art_can: PhantomData,
			_b_can: PhantomData,
		}
	}
}

impl<ArtCan, BCan, F: 'static, T: Debug + 'static> From<F> for Blueprint<FunctionalBuilder<ArtCan, BCan, F>, BCan>
	where
		F: (for<'r, 's> Fn(&'r mut Resolver<'s, ArtCan, BCan>) -> T) + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		BCan: CanSized<FunctionalBuilder<ArtCan, BCan, F>>,
		ArtCan: 'static {

	fn from(f: F) -> Self {
		Blueprint::new(
			FunctionalBuilder::new(f)
		)
	}
}

impl<ArtCan, BCan, F, T> Builder<ArtCan, BCan> for FunctionalBuilder<ArtCan, BCan, F>
	where
		F: (for<'r, 's> Fn(&'r mut Resolver<'s, ArtCan, BCan>) -> T) + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		ArtCan: 'static {

	type Artifact = T;
	type DynState = ();
	type Err = Never;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan>)
			 -> Result<Self::Artifact, Never> {

		let f = &self.inner;
		Ok(f(resolver))

	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}




/// A static builder.
///
/// A builder which always builds a predetermined value as artifact.
///
pub struct ConstBuilder<ArtCan, BCan, T> {
	inner: T,
	_art_can: PhantomData<ArtCan>,
	_b_can: PhantomData<BCan>,
}

impl<ArtCan, BCan, T: Debug> Debug for ConstBuilder<ArtCan, BCan, T> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "FunctionalBuilder{{inner: {:?}}}", self.inner)
	}
}

impl<ArtCan, BCan, T> ConstBuilder<ArtCan, BCan, T>
	where
		T: Clone + Debug + 'static,
		BCan: CanStrong,
		ArtCan: 'static {

	/// Wraps the given closure as Builder.
	///
	pub fn new(artifact: T) -> Self {
		ConstBuilder {
			inner: artifact,
			_art_can: PhantomData,
			_b_can: PhantomData,
		}
	}
}

impl<ArtCan, BCan, T> From<T> for Blueprint<ConstBuilder<ArtCan, BCan, T>, BCan>
	where
		T: Clone + Debug + 'static,
		BCan: CanStrong,
		BCan: CanSized<ConstBuilder<ArtCan, BCan, T>>,
		ArtCan: 'static {

	fn from(t: T) -> Self {
		Blueprint::new(
			ConstBuilder::new(t)
		)
	}
}

impl<ArtCan, BCan, T> Builder<ArtCan, BCan> for ConstBuilder<ArtCan, BCan, T>
	where
		T: Clone + Debug + 'static,
		BCan: CanStrong,
		ArtCan: 'static {

	type Artifact = T;
	type DynState = ();
	type Err = Never;

	fn build(&self, _resolver: &mut Resolver<ArtCan, BCan>)
			 -> Result<Self::Artifact, Never> {

		Ok(self.inner.clone())
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}




/// A dynamic builder.
///
/// A `ConfigurableBuilder` is a builder which's artifact can be reconfigured
/// i.e. changed by changing it's dyn state.
///
pub struct ConfigurableBuilder<ArtCan, BCan, T> {
	initial: T,
	_art_can: PhantomData<ArtCan>,
	_b_can: PhantomData<BCan>,
}

impl<ArtCan, BCan, T: Debug> Debug for ConfigurableBuilder<ArtCan, BCan, T> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "FunctionalBuilder{{initial: {:?}}}", self.initial)
	}
}

impl<ArtCan, BCan, T> ConfigurableBuilder<ArtCan, BCan, T>
	where
		T: Clone + Debug + 'static,
		BCan: CanStrong,
		ArtCan: Debug + 'static {

	/// Wraps the given closure as Builder.
	///
	pub fn new(artifact: T) -> Self {
		ConfigurableBuilder {
			initial: artifact,
			_art_can: PhantomData,
			_b_can: PhantomData,
		}
	}
}

impl<ArtCan, BCan, T> From<T> for Blueprint<ConfigurableBuilder<ArtCan, BCan, T>, BCan>
	where
		T: Clone + Debug + 'static,
		BCan: CanStrong,
		BCan: CanSized<ConfigurableBuilder<ArtCan, BCan, T>>,
		ArtCan: Debug + 'static {

	fn from(t: T) -> Self {
		Blueprint::new(
			ConfigurableBuilder::new(t)
		)
	}
}

impl<ArtCan, BCan, T> Builder<ArtCan, BCan> for ConfigurableBuilder<ArtCan, BCan, T>
	where
		T: Clone + Debug + 'static,
		BCan: CanStrong,
		ArtCan: Debug + 'static {

	type Artifact = T;
	type DynState = T;
	type Err = Never;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, T>)
			 -> Result<Self::Artifact, Never> {

		Ok(resolver.my_state().clone())
	}
	fn init_dyn_state(&self) -> Self::DynState {
		self.initial.clone()
	}
}




/// A intermediate Builder without dyn state.
///
/// When different builder shall be hidden behind a `dyn Builder` it is required
/// that all such builders have the same dyn state type. Thus, often non dyn
/// state or the `()` unit type as dyn state is used. This wrapper build now
/// allows to wrap arbitrary builders (e.g. those with a dyn state) into a
/// builder that dose has the `()` unit type as dyn state. So that it may be
/// use to create cast it into `dyn Builder` with the `()` unit type as dyn state.
///
/// However, in order to create a valid artifact, the artifact type must be
/// `Clone`.
///
/// Also see the `ForwardingBuilder` for an alternative.
///
#[derive(Debug, Clone)]
pub struct ClonedBuilder<AP, B: ?Sized> {
	inner: AP,
	_b: PhantomData<B>,
}

impl<AP, B: ?Sized> ClonedBuilder<AP, B>
	where
		B: Debug + 'static, {

	/// Wrap given Builder cloning its artifact.
	///
	pub fn new<ArtCan, BCan>(
		inner: AP,
	) -> Blueprint<Self, BCan>
		where
			B: Builder<ArtCan, BCan>,
			B::Artifact: Clone,
			AP: Promise<B, BCan>,
			ArtCan: CanSized<B::Artifact> + CanRef<B::Artifact>,
			ArtCan::Bin: AsRef<B::Artifact>,
			BCan: Clone + CanStrong,
			BCan: CanSized<Self>,
	{

		Blueprint::new(
			ClonedBuilder {
				inner,
				_b: PhantomData,
			}
		)
	}
}

impl<ArtCan, AP, B: ?Sized, BCan> Builder<ArtCan, BCan> for ClonedBuilder<AP, B>
	where
		B: Builder<ArtCan, BCan>,
		B::Artifact: Clone,
		AP: Promise<B, BCan>,
		ArtCan: CanRef<B::Artifact>,
		BCan: CanStrong,
	{

	type Artifact = B::Artifact;
	type DynState = ();
	type Err = B::Err;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			-> Result<Self::Artifact, Self::Err> {

		resolver.resolve_cloned(&self.inner)
	}

	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}



/// A intermediate Builder without dyn state.
///
/// When different builder shall be hidden behind a `dyn Builder` it is required
/// that all such builders have the same dyn state type. Thus, often non dyn
/// state or the `()` unit type as dyn state is used. This wrapper builder now
/// allows to wrap arbitrary builders (e.g. those with a dyn state) into a
/// builder that dose has the `()` unit type as dyn state. So that it may be
/// use to create cast it into `dyn Builder` with the `()` unit type as dyn state.
///
/// However, in order to create a valid artifact, the artifact is kept in its bin-state.
///
/// Also see the `ClonedBuilder` for an alternative.
///
#[derive(Debug, Clone)]
pub struct ForwardingBuilder<AP, B: ?Sized> {
	inner: AP,
	_b: PhantomData<B>,
}

impl<AP, B: ?Sized> ForwardingBuilder<AP, B>
	where
		B: Debug + 'static, {

	/// Wrap given Builder forwarding its artifact.
	///
	pub fn new<ArtCan, BCan>(
		inner: AP,
	) -> Blueprint<Self, BCan>
		where
			B: Builder<ArtCan, BCan>,
			AP: Promise<B, BCan>,
			ArtCan: CanSized<B::Artifact>,
			ArtCan: Clone,
			BCan: CanStrong,
			BCan: CanSized<Self>,
	{

		Blueprint::new(
			ForwardingBuilder {
				inner,
				_b: PhantomData,
			}
		)
	}
}

impl<ArtCan, AP, B: ?Sized, BCan> Builder<ArtCan, BCan> for ForwardingBuilder<AP, B>
	where
		B: Builder<ArtCan, BCan>,
		AP: Promise<B, BCan>,
		ArtCan: CanSized<B::Artifact>,
		ArtCan: Clone,
		BCan: CanStrong,
	{

	type Artifact = ArtCan::Bin;
	type DynState = ();
	type Err = B::Err;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			-> Result<Self::Artifact, Self::Err> {

		resolver.resolve(&self.inner)
	}

	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}





