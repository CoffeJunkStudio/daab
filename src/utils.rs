

use crate::ArtifactResolver;
use crate::ArtifactPromiseTrait;
use crate::ArtifactPromise;
use crate::ArtifactCache;
use crate::Builder;
use crate::Can;
use crate::CanRef;
use crate::CanStrong;
use crate::CanSized;

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
#[derive(Debug, Clone)]
pub struct FallibleBuilder<AP, B: ?Sized, T> {
	inner: AP,
	_b: PhantomData<B>,
	_t: PhantomData<T>,
}

impl<AP, B: ?Sized, T> FallibleBuilder<AP, B, T>
	where
		B: Debug + 'static, {

	/// Wrap given Builder and fill missing recreations with a previous value.
	///
	/// **Use with care**
	///
	pub fn new<ArtCan, BCan>(
		cache: &mut ArtifactCache<ArtCan, BCan>,
		inner: AP,
		default_value: Option<T>
	) -> ArtifactPromise<Self, BCan>
	where
		B: Builder<ArtCan, BCan, Artifact=Option<T>> + Debug + 'static,
		AP: ArtifactPromiseTrait<B, BCan> + Debug + 'static,
		T: Clone + Debug + 'static,
		ArtCan: Debug,
		ArtCan: Can<T>,
		ArtCan: CanSized<Option<T>> + CanRef<Option<T>>,
		<ArtCan as Can<Option<T>>>::Bin: AsRef<Option<T>>,
		BCan: Clone + Debug + CanStrong,
		BCan: CanSized<Self>,
		<BCan as Can<Self>>::Bin: Clone + AsRef<Self>,
		BCan: Can<B>,
		<BCan as Can<B>>::Bin: Clone + AsRef<B>,
		{

		let me = ArtifactPromise::new(
			FallibleBuilder {
				inner,
				_b: PhantomData,
				_t: PhantomData,
			}
		);

		cache.set_dyn_state(&me, default_value);

		me
	}
}

impl<ArtCan, AP, B: ?Sized, BCan, T> Builder<ArtCan, BCan> for FallibleBuilder<AP, B, T>
	where
		B: Builder<ArtCan, BCan, Artifact=Option<T>> + Debug + 'static,
		AP: ArtifactPromiseTrait<B, BCan> + Debug,
		T: Clone + Debug + 'static,
		ArtCan: Debug,
		ArtCan: CanSized<Option<T>> + CanRef<Option<T>>,
		ArtCan::Bin: AsRef<Option<T>>,
		BCan: Clone + Debug + CanStrong,
		BCan: Can<B>,
		<BCan as Can<B>>::Bin: Clone + AsRef<B>,
	{

	type Artifact = T;
	type DynState = Option<T>;

	fn build(&self, resolver: &mut ArtifactResolver<ArtCan, BCan, Self::DynState>) -> Self::Artifact {

		let value = resolver.resolve_cloned(&self.inner);

		if let Some(v) = value {
			*resolver.my_state() = Some(v.clone());

			// Return value
			v

		} else {
			// Try to return cached value. Panics if very first build fails.
			resolver.my_state().clone().unwrap()
		}
	}
}

