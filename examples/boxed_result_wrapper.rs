
use daab::boxed::wrapped::ArtifactCacheWrapped as Cache;
use daab::boxed::wrapped::ArtifactResolverWrapped as Resolver;
use daab::boxed::wrapped::BuilderWrapped as Builder;
use daab::boxed::ArtifactPromise as Ap;

use std::rc::Rc;

#[derive(Debug)]
struct FooArtifact;

#[derive(Debug)]
struct BazArtifact;

#[derive(Debug)]
struct BarArtifact {
	foo_artifact: Rc<FooArtifact>,
	baz_artifact: Rc<BazArtifact>
}

#[derive(Debug)]
struct FooBuilder;

impl Builder for FooBuilder {
	type Artifact = Result<Rc<FooArtifact>, ()>;
	type DynState = ();

	fn build(&self, resolver: &mut Resolver) -> Self::Artifact {
		println!("Building FooArtifact...");
		Ok(FooArtifact.into())
	}
}

#[derive(Debug)]
struct BazBuilder;

impl Builder for BazBuilder {
	type Artifact = Rc<BazArtifact>;
	type DynState = ();

	fn build(&self, resolver: &mut Resolver) -> Self::Artifact {
		println!("Building BazArtifact...");
		BazArtifact.into()
	}
}

#[derive(Debug)]
struct BarBuilder {
	foo_builder: Ap<FooBuilder>,
	baz_builder: Ap<BazBuilder>
}

impl Builder for BarBuilder {
	type Artifact = Result<Rc<BarArtifact>, ()>;
	type DynState = ();

	fn build(&self, resolver: &mut Resolver) -> Self::Artifact {
		let foo_artifact = resolver.resolve(&self.foo_builder)?;
		let baz_artifact = resolver.resolve(&self.baz_builder);
		println!("Building BarArtifact...");
		Ok(BarArtifact {
			foo_artifact,
			baz_artifact
		}.into())
	}
}

fn main() {
	let mut cache = Cache::new();

	// Simple hack to make diagnostics work
	let cache: &mut Cache = &mut cache;

	let foo_builder = Ap::new(FooBuilder);
	let baz_builder = Ap::new(BazBuilder);
	let bar_builder = Ap::new(BarBuilder {
		foo_builder: foo_builder.clone(),
		baz_builder
	});

	println!("Taking FooBuilder dyn_state...");
	let dyn_st = cache.get_dyn_state(&foo_builder);
	dbg!(dyn_st);
	println!("Setting FooBuilder dyn_state...");
	cache.set_dyn_state(&foo_builder, ());
	println!("Taking FooBuilder dyn_state...");
	let dyn_st = cache.get_dyn_state(&foo_builder);
	dbg!(dyn_st);
	println!("Taking FooBuilder dyn_state...");
	let dyn_st = cache.get_dyn_state(&foo_builder);
	dbg!(dyn_st);

	println!("Requesting BarArtifact clone...");
	cache.get(&bar_builder);
	println!("Requesting BarArtifact ref...");
	cache.get_ref(&bar_builder);
	println!("Invalidating BarArtifact...");
	cache.invalidate(&bar_builder);
	println!("Requesting BarArtifact clone...");
	cache.get_cloned(&bar_builder);
}

