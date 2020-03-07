


use std::fmt::Debug;
use std::any::Any;

#[cfg(feature = "diagnostics")]
use crate::Doctor;

use crate::BuilderEntry;


type BinType<T> = std::rc::Rc<T>;
type CanType = BinType<dyn Any>;


pub type ArtifactPromise<B> = crate::ArtifactPromise<B, CanType>;

pub type ArtifactResolver<'a, T = ()> = crate::ArtifactResolver<'a, CanType, CanType, T>;

pub type SuperArtifactResolver<'a, T = ()> = crate::ArtifactResolver<'a, BuilderEntry<CanType>, CanType, T>;


#[cfg(not(feature = "diagnostics"))]
pub type ArtifactCache = crate::ArtifactCache<CanType, CanType>;

#[cfg(feature = "diagnostics")]
pub type ArtifactCache<T = dyn Doctor<CanType, CanType>> = crate::ArtifactCache<CanType, CanType, T>;

#[cfg(not(feature = "diagnostics"))]
pub type SuperArtifactCache = crate::ArtifactCache<BuilderEntry<CanType>, CanType>;

#[cfg(feature = "diagnostics")]
pub type SuperArtifactCache<T = dyn Doctor<BuilderEntry<CanType>, CanType>> = crate::ArtifactCache<BuilderEntry<CanType>, CanType, T>;


pub trait SimpleBuilder: Debug {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;
	
	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut ArtifactResolver) -> Self::Artifact;
}

// Generic impl for legacy builder
impl<B: SimpleBuilder> Builder for B {
	type Artifact = B::Artifact;
	
	type DynState = ();
	
	fn build(&self, cache: &mut ArtifactResolver) -> BinType<Self::Artifact> {
		BinType::new(self.build(cache))
	}
}


pub trait Builder: Debug {
	type Artifact : Debug + 'static;
	
	type DynState : Debug + 'static;
	
	fn build(&self, resolver: &mut ArtifactResolver<Self::DynState>) -> BinType<Self::Artifact>;
}

impl<B: Builder> crate::Builder<CanType, CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;
	
	fn build(&self, cache: &mut ArtifactResolver<Self::DynState>) -> BinType<Self::Artifact> {
		self.build(cache)
	}
}

pub trait SuperBuilder: Debug {
	type Artifact : Debug + 'static;
	
	type DynState : Debug + 'static;
	
	fn build(&self, resolver: &mut SuperArtifactResolver<Self::DynState>) -> ArtifactPromise<Self::Artifact>;
}

impl<B: SuperBuilder> crate::Builder<crate::BuilderEntry<CanType>, CanType> for B {
	type Artifact = B::Artifact;
	type DynState = B::DynState;
	
	fn build(&self, cache: &mut SuperArtifactResolver<Self::DynState>) -> ArtifactPromise<Self::Artifact> {
		self.build(cache)
	}
}



