


use crate::*;


pub type ArtifactPromiseRc<B> = ArtifactPromise<B, Rc<dyn Any>>;

pub type ArtifactResolverRc<'a, T = ()> = ArtifactResolver<'a, Rc<dyn Any>, Rc<dyn Any>, T>;



#[cfg(not(feature = "diagnostics"))]
pub type ArtifactCacheRc = ArtifactCache<Rc<dyn Any>, Rc<dyn Any>>;

#[cfg(feature = "diagnostics")]
pub type ArtifactCacheRc<T = dyn Doctor<Rc<dyn Any>, Rc<dyn Any>>> = ArtifactCache<Rc<dyn Any>, Rc<dyn Any>, T>;


pub trait SimpleBuilder: Debug {
	/// The artifact type as produced by this builder.
	///
	type Artifact : Debug + 'static;
	
	/// Produces an artifact using the given `ArtifactResolver` for resolving
	/// dependencies.
	///
	fn build(&self, resolver: &mut ArtifactResolverRc) -> Self::Artifact;
}

// Generic impl for legacy builder
impl<B: SimpleBuilder> BuilderRc for B {
	type Artifact = B::Artifact;
	
	type UserData = ();
	
	fn build(&self, cache: &mut ArtifactResolverRc) -> Rc<Self::Artifact> {
		Rc::new(self.build(cache))
	}
}


pub trait BuilderRc: Debug {
	type Artifact : Debug + 'static;
	
	type UserData : Debug + 'static;
	
	fn build(&self, resolver: &mut ArtifactResolverRc<Self::UserData>) -> Rc<Self::Artifact>;
}

impl<B: BuilderRc> BuilderWithData<Rc<dyn Any>, Rc<dyn Any>> for B {
	type Artifact = B::Artifact;
	type UserData = B::UserData;
	
	fn build(&self, cache: &mut ArtifactResolverRc<Self::UserData>) -> Rc<Self::Artifact> {
		self.build(cache)
	}
}


