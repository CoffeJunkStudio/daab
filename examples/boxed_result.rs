
use daab::boxed::Cache;
use daab::boxed::Resolver;
use daab::boxed::Blueprint as Bp;
use daab::boxed::Builder as Builder;

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
	foo_builder: Bp<FooBuilder>,
	baz_builder: Bp<BazBuilder>
}

impl Builder for BarBuilder {
	type Artifact = Result<Rc<BarArtifact>, ()>;
	type DynState = ();

	fn build(&self, resolver: &mut Resolver) -> Self::Artifact {
		let foo_artifact = resolver.resolve_ref(&self.foo_builder).clone()?;
		let baz_artifact = resolver.resolve_ref(&self.baz_builder).clone();
		println!("Building BarArtifact...");
		Ok(BarArtifact {
			foo_artifact,
			baz_artifact
		}.into())
	}
}

fn main() {
	let mut cache = Cache::new();

	let foo_builder = Bp::new(FooBuilder);
	let baz_builder = Bp::new(BazBuilder);
	let bar_builder = Bp::new(BarBuilder {
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

	println!("Requesting BarArtifact...");
	cache.get_ref(&bar_builder);
	println!("Requesting BarArtifact...");
	cache.get_ref(&bar_builder);
	println!("Invalidating BarArtifact...");
	cache.invalidate(&bar_builder);
	println!("Requesting BarArtifact...");
	cache.get_ref(&bar_builder);
	println!("Requesting BarArtifact...");
	cache.get_ref(&bar_builder);
}

