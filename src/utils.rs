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
use crate::Can;
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
/// Notice, that once the inner builder faild, and the `RedeemingBuilder` 'built' the _old artifact_
/// then this will not establish a dependency on the inner builder. So to make the
/// `RedeemingBuilder` return a 'fresh' artifact from the inner builder, the `RedeemingBuilder`
/// itself needs to be invalidated. This should otherwise not be an issue, because if the inner
/// builder still fails, the `RedeemingBuilder` will again retun the _old artifact_.
///
/// # Panics
///
/// This builder panics in its `build` method if the first build of its inner
/// builder failed and the `default_value` has been set to `None`.
///
#[derive(Debug, Clone)]
pub struct RedeemingBuilder<AP, ArtBin> {
	inner: AP,
	default_value: Option<ArtBin>,
}

impl<AP, ArtBin> RedeemingBuilder<AP, ArtBin> {

	/// Wrap given Builder and fill missing recreations with a previous value.
	///
	/// **Use with care**
	///
	pub fn new<ArtCan, BCan, B: ?Sized, T>(
		inner: AP,
		default_value: Option<ArtBin>
	) -> Self
		where
			B: Builder<ArtCan, BCan, Artifact=T>,
			BCan: Can<AP::Builder>,
			AP: Promise<Builder = B, BCan = BCan>,
			T: Debug + 'static,
			ArtCan: Clone + CanSized<T,Bin=ArtBin>,
			ArtBin: Clone + Debug + 'static,
			BCan: Clone + CanStrong,
			BCan: CanSized<Self>,
	{

		RedeemingBuilder {
			inner,
			default_value,
		}
	}
}

impl<ArtCan, AP, B: ?Sized, BCan, ArtBin, T> Builder<ArtCan, BCan> for RedeemingBuilder<AP, ArtBin>
	where
		B: Builder<ArtCan, BCan, Artifact=T>,
		BCan: Can<B>,
		AP: Promise<Builder = B, BCan = BCan>,
		T: Debug + 'static,
		ArtCan: Clone + CanSized<T,Bin=ArtBin>,
		ArtBin: Clone + Debug + 'static,
		BCan: Clone + CanStrong,
	{

	type Artifact = T;
	type DynState = Option<ArtCan::Bin>;
	type Err = Never;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			-> Result<ArtCan::Bin, Never> {

		let value = resolver.resolve(&self.inner);

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




/// Functional leaf builder wrapper.
///
/// A functional builder is a builder consisting of a single function
/// `Fn(&mut S) -> Result<ArtCan::Bin,E>`. Thus this type can be used to wrap a
/// closure as `Builder` with type `T` as the artifact type, and type `E` as error type.
/// The closure takes a mutable reference to a `S` (default `()`) which is the DynState
/// of the builder. However, the closure will not have a `Resolver` thus it can not
/// depend on other builders, making it a 'leaf' builder. So it is most useful as a
/// 'provider' builder.
///
/// Also see `ConstBuilder` and `ConfigurableBuilder` for alternatives.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::rc::Rc;
/// use daab::utils::FunctionalBuilder;
/// use daab::rc::Cache;
/// use daab::rc::Blueprint;
/// use daab::prelude::*;
///
/// let builder = FunctionalBuilder::new(
///     |_| {
///         Ok(Rc::new(42))
///     }
/// );
/// let blueprint = Blueprint::new(builder);
///
/// let mut cache = Cache::new();
///
/// assert_eq!(42_u32, cache.get_cloned(&blueprint).unpack());
/// ```
///
/// Advanced usage with DynState:
///
/// ```
/// use std::rc::Rc;
/// use daab::utils::FunctionalBuilder;
/// use daab::rc::Cache;
/// use daab::rc::Blueprint;
///
/// let builder = FunctionalBuilder::with_state(0_u32, |st| {
///     if *st < 2 {
///         *st += 1;
///         Ok(Rc::new(*st))
///     } else {
///         Err(false)
///     }
/// });
/// let blueprint = Blueprint::new(builder);
///
/// let mut cache = Cache::new();
///
/// assert_eq!(Ok(1_u32), cache.get_cloned(&blueprint));
/// # assert_eq!(Ok(1_u32), cache.get_cloned(&blueprint));
/// cache.invalidate(&blueprint);
/// assert_eq!(Ok(2_u32), cache.get_cloned(&blueprint));
/// cache.invalidate(&blueprint);
/// assert_eq!(Err(false), cache.get_cloned(&blueprint));
/// ```
///
/// Real world scenario with `File`:
///
/// ```
/// use std::fs::File;
/// use std::rc::Rc;
/// use daab::utils::FunctionalBuilder;
/// use daab::rc::Cache;
/// use daab::rc::CanType;
/// use daab::rc::Blueprint;
///
/// let builder = FunctionalBuilder::new(
///     |_| {
///         File::open("some_path").map(Rc::new)
///     }
/// );
/// let blueprint = Blueprint::new(builder);
///
/// let mut cache = Cache::new();
///
/// let file = cache.get(&blueprint);
///
/// # assert!(file.is_err());
/// ```
///
pub struct FunctionalBuilder<ArtCan, BCan, F, T, S = ()> {
	inner: F,
	initial_state: S,
	_art_can: PhantomData<ArtCan>,
	_b_can: PhantomData<BCan>,
	_t: PhantomData<T>,
}

impl<ArtCan, BCan, F, T, S> Debug for FunctionalBuilder<ArtCan, BCan, F, T, S> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "FunctionalBuilder{{...}}")
	}
}

impl<ArtCan, BCan, F, E, T> FunctionalBuilder<ArtCan, BCan, F, T, ()>
	where
		F: (for<'r> Fn( &'r mut () ) -> Result<ArtCan::Bin,E>) + 'static,
		E: Debug + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		ArtCan: Can<T>,
		ArtCan: Debug + 'static {

	/// Wraps the given closure as Builder.
	///
	pub fn new(f: F) -> Self {
		FunctionalBuilder::with_state( (), f )
	}
}

impl<ArtCan, BCan, F, E, T, S> FunctionalBuilder<ArtCan, BCan, F, T, S>
	where
		F: (for<'r> Fn(&'r mut S) -> Result<ArtCan::Bin,E>) + 'static,
		E: Debug + 'static,
		T: Debug + 'static,
		S: Clone + Debug + 'static,
		BCan: CanStrong,
		ArtCan: Can<T>,
		ArtCan: Debug + 'static {

	/// Wraps the given closure as Builder.
	///
	pub fn with_state(initial_state: S, f: F) -> Self {
		FunctionalBuilder {
			inner: f,
			initial_state,
			_art_can: PhantomData,
			_b_can: PhantomData,
			_t: PhantomData,
		}
	}
}

/*
/// Convert a 'stateless' closure into a `FunctionalBuilder`
impl<ArtCan, BCan, F, E, T> From<F> for FunctionalBuilder<ArtCan, BCan, F, ()>
	where
		F: (for<'r> Fn( &'r mut () ) -> Result<T,E>) + 'static,
		E: Debug + 'static,
		T: Debug + 'static,
		BCan: CanStrong,
		BCan: CanSized<FunctionalBuilder<ArtCan, BCan, F, ()>>,
		ArtCan: Debug + 'static,
		Self: Builder<ArtCan, BCan> {

	fn from(f: F) -> Self {
		FunctionalBuilder::with_state((), f)
	}
}
*/

impl<ArtCan, BCan, F, E, T, S> Builder<ArtCan, BCan> for FunctionalBuilder<ArtCan, BCan, F, T, S>
	where
		F: (for<'r> Fn( &'r mut S ) -> Result<ArtCan::Bin,E>) + 'static,
		E: Debug + 'static,
		T: Debug + 'static,
		S: Clone + Debug + 'static,
		BCan: CanStrong,
		ArtCan: Can<T>,
		ArtCan: Debug + 'static {

	type Artifact = T;
	type DynState = S;
	type Err = E;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			 -> Result<ArtCan::Bin, Self::Err> {

		let f = &self.inner;
		let state = resolver.my_state();

		f(state)

	}
	fn init_dyn_state(&self) -> Self::DynState {
		self.initial_state.clone()
	}
}




/// A static builder.
///
/// A builder which always builds a predetermined value as artifact.
///
/// Also see `FunctionalBuilder` and `ConfigurableBuilder` for alternatives.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use std::rc::Rc;
/// use daab::utils::ConstBuilder;
/// use daab::rc::Cache;
/// use daab::rc::Blueprint;
/// use daab::prelude::*;
///
/// let builder = ConstBuilder::new(Rc::new(42_u32));
/// let blueprint = Blueprint::new(builder);
///
/// let mut cache = Cache::new();
///
/// assert_eq!(42_u32, cache.get_cloned(&blueprint).unpack());
/// # cache.invalidate(&blueprint);
/// # assert_eq!(42_u32, cache.get_cloned(&blueprint).unpack());
/// ```
///
pub struct ConstBuilder<ArtCan, BCan, ArtBin, T> {
	inner: ArtBin,
	_art_can: PhantomData<ArtCan>,
	_b_can: PhantomData<BCan>,
	_t: PhantomData<T>,
}

impl<ArtCan, BCan, ArtBin: Debug, T> Debug for ConstBuilder<ArtCan, BCan, ArtBin, T> {
	fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
		write!(fmt, "FunctionalBuilder{{inner: {:?}}}", self.inner)
	}
}

impl<ArtCan, BCan, ArtBin, T> ConstBuilder<ArtCan, BCan, ArtBin, T>
	where
		BCan: CanStrong,
		ArtCan: Can<T,Bin=ArtBin>,
		ArtCan: 'static,
		ArtBin: Clone + Debug + 'static, {

	/// Wraps the given closure as Builder.
	///
	pub fn new(artifact_bin: ArtBin) -> Self {

		ConstBuilder {
			inner: artifact_bin,
			_art_can: PhantomData,
			_b_can: PhantomData,
			_t: PhantomData,
		}
	}
}

impl<ArtCan, BCan, ArtBin, T> From<T> for Blueprint<ConstBuilder<ArtCan, BCan, ArtBin, T>, BCan>
	where
		T: Clone + Debug + 'static,
		BCan: CanStrong,
		BCan: CanSized<ConstBuilder<ArtCan, BCan, ArtBin, T>>,
		ArtCan: CanSized<T,Bin=ArtBin>,
		ArtCan: 'static,
		ArtBin: Clone + Debug + 'static, {

	fn from(t: T) -> Self {
		Blueprint::new(
			ConstBuilder::new(ArtCan::into_bin(t))
		)
	}
}

impl<ArtCan, BCan, ArtBin, T> Builder<ArtCan, BCan> for ConstBuilder<ArtCan, BCan, ArtBin, T>
	where
		T: Debug + 'static,
		BCan: CanStrong,
		ArtCan: Can<T,Bin=ArtBin>,
		ArtCan: 'static,
		ArtBin: Clone + Debug + 'static, {

	type Artifact = T;
	type DynState = ();
	type Err = Never;

	fn build(&self, _resolver: &mut Resolver<ArtCan, BCan>)
			 -> Result<ArtBin, Never> {

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
/// Also see `FunctionalBuilder` and `ConstBuilder` for alternatives.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use daab::utils::ConfigurableBuilder;
/// use daab::rc::Cache;
/// use daab::rc::Blueprint;
/// use daab::prelude::*;
///
/// let builder = ConfigurableBuilder::new(0_u32);
/// let blueprint = Blueprint::new(builder);
///
/// let mut cache = Cache::new();
///
/// assert_eq!(0_u32, cache.get_cloned(&blueprint).unpack());
/// *cache.dyn_state_mut(&blueprint) = 42;
/// assert_eq!(42_u32, cache.get_cloned(&blueprint).unpack());
/// # cache.invalidate(&blueprint);
/// # assert_eq!(42_u32, cache.get_cloned(&blueprint).unpack());
/// ```
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
		ArtCan: CanSized<T>,
		ArtCan: Debug + 'static {

	type Artifact = T;
	type DynState = T;
	type Err = Never;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, T>)
			 -> Result<ArtCan::Bin, Never> {

		Ok(ArtCan::into_bin(resolver.my_state().clone()))
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
/// `Clone`. Thus the original and the 'cloned' artifact are not identical.
///
/// Also see the `ForwardingBuilder` for an alternative.
///
#[derive(Debug, Clone)]
pub struct ClonedBuilder<AP> {
	inner: AP,
}

impl<AP> ClonedBuilder<AP> {

	/// Wrap given Builder cloning its artifact.
	///
	pub fn new<ArtCan, BCan, B: ?Sized>(
		inner: AP,
	) -> Self
		where
			B: Builder<ArtCan, BCan>,
			B::Artifact: Clone,
			BCan: Can<AP::Builder>,
			AP: Promise<Builder = B, BCan = BCan>,
			ArtCan: CanRef<B::Artifact>,
			BCan: Clone + CanStrong,
			BCan: CanSized<Self>,
	{

		ClonedBuilder {
			inner,
		}
	}
}

impl<ArtCan, AP, B: ?Sized, BCan> Builder<ArtCan, BCan> for ClonedBuilder<AP>
	where
		B: Builder<ArtCan, BCan>,
		B::Artifact: Clone,
		BCan: Can<B>,
		AP: Promise<Builder = B, BCan = BCan>,
		ArtCan: CanRef<B::Artifact>,
		BCan: CanStrong,
	{

	type Artifact = B::Artifact;
	type DynState = ();
	type Err = B::Err;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			-> Result<ArtCan::Bin, Self::Err> {

		resolver.resolve_cloned(&self.inner)
			.map(ArtCan::into_bin)
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
pub struct ForwardingBuilder<AP> {
	inner: AP,
}

impl<AP> ForwardingBuilder<AP> {

	/// Wrap given Builder forwarding its artifact.
	///
	pub fn new<ArtCan, BCan, B: ?Sized>(
		inner: AP,
	) -> Self
		where
			B: Builder<ArtCan, BCan>,
			BCan: Can<AP::Builder>,
			AP: Promise<Builder = B, BCan = BCan>,
			ArtCan: CanSized<B::Artifact>,
			ArtCan: Clone,
			BCan: CanStrong,
			BCan: CanSized<Self>,
	{

		ForwardingBuilder {
			inner,
		}
	}
}

impl<ArtCan, AP, B: ?Sized, BCan> Builder<ArtCan, BCan> for ForwardingBuilder<AP>
	where
		B: Builder<ArtCan, BCan>,
		BCan: Can<B>,
		AP: Promise<Builder = B, BCan = BCan>,
		ArtCan: CanSized<B::Artifact>,
		ArtCan: Clone,
		BCan: CanStrong,
	{

	type Artifact = B::Artifact;
	type DynState = ();
	type Err = B::Err;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			-> Result<ArtCan::Bin, Self::Err> {

		resolver.resolve(&self.inner)
	}

	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}



/// A intermediate Builder which wraps a builder with `Err=Never` with a arbitrary error type.
///
#[derive(Debug, Clone)]
pub struct FeigningBuilder<AP, Err> {
	inner: AP,
	_err: PhantomData<Err>,
}

impl<AP, Err> FeigningBuilder<AP, Err> {

	/// Wrap given Builder forwarding its artifact.
	///
	pub fn new<ArtCan, BCan, B>(
		inner: AP,
	) -> Self
		where
			B: Builder<ArtCan, BCan, Err=Never>,
			BCan: Can<AP::Builder>,
			AP: Promise<Builder = B, BCan = BCan>,
			Err: Debug + 'static,
			ArtCan: CanSized<B::Artifact>,
			ArtCan: Clone,
			BCan: CanStrong,
			BCan: CanSized<Self>,
	{

		FeigningBuilder {
			inner,
			_err: PhantomData,
		}
	}
}

impl<ArtCan, AP, B: ?Sized, BCan, Err> Builder<ArtCan, BCan> for FeigningBuilder<AP, Err>
	where
		B: Builder<ArtCan, BCan, Err=Never>,
		BCan: Can<B>,
		AP: Promise<Builder = B, BCan = BCan>,
		Err: Debug + 'static,
		ArtCan: CanSized<B::Artifact>,
		ArtCan: Clone,
		BCan: CanStrong,
	{

	type Artifact = B::Artifact;
	type DynState = ();
	type Err = Err;

	fn build(&self, resolver: &mut Resolver<ArtCan, BCan, Self::DynState>)
			-> Result<ArtCan::Bin, Self::Err> {

		resolver.resolve(&self.inner).map_err(|n| n.into_any())
	}

	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}





