
use daab::rc::Cache;
use daab::rc::Resolver;
use daab::rc::Blueprint as Bp;
use daab::rc::Builder;
use daab::Never;
use daab::prelude::*;

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
	type Artifact = FooArtifact;
	type DynState = ();
	type Err = ();

	fn build(&self, _resolver: &mut Resolver) -> Result<Rc<Self::Artifact>, ()> {
		println!("Building FooArtifact...");
		Ok(Rc::new(FooArtifact))
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}

#[derive(Debug)]
struct BazBuilder;

impl Builder for BazBuilder {
	type Artifact = BazArtifact;
	type DynState = ();
	type Err = Never;

	fn build(&self, _resolver: &mut Resolver) -> Result<Rc<Self::Artifact>, Never> {
		println!("Building BazArtifact...");
		Ok(BazArtifact.into())
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}

#[derive(Debug)]
struct BarBuilder {
	foo_builder: Bp<FooBuilder>,
	baz_builder: Bp<BazBuilder>
}

impl Builder for BarBuilder {
	type Artifact = BarArtifact;
	type DynState = ();
	type Err = ();

	fn build(&self, resolver: &mut Resolver) -> Result<Rc<Self::Artifact>, ()> {
		let foo_artifact = resolver.resolve(&self.foo_builder)?;
		let baz_artifact = resolver.resolve(&self.baz_builder).unpack();
		println!("Building BarArtifact...");
		Ok(BarArtifact {
			foo_artifact,
			baz_artifact
		}.into())
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
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
	*cache.dyn_state_mut(&foo_builder) = ();
	println!("Taking FooBuilder dyn_state...");
	let dyn_st = cache.get_dyn_state(&foo_builder);
	dbg!(dyn_st);
	println!("Taking FooBuilder dyn_state...");
	let dyn_st = cache.get_dyn_state(&foo_builder);
	dbg!(dyn_st);

	println!("Requesting BarArtifact...");
	cache.get(&bar_builder).unwrap();
	println!("Requesting BarArtifact...");
	cache.get(&bar_builder).unwrap();
	println!("Invalidating BarArtifact...");
	cache.invalidate(&bar_builder);
	println!("Requesting BarArtifact...");
	cache.get(&bar_builder).unwrap();
}

