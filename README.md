[![Crates.io](https://img.shields.io/crates/v/daab.svg)](https://crates.io/crates/daab)


DAG Aware Artifact Builder
==========================

Rust crate for managing the building and caching of artifacts which are
connected in a directed acyclic graph (DAG) like manner, i.e. artifacts may
depend on others.

The caching provided by this crate could be especially useful if the
artifact builders use consumable resources, the building process is a
heavyweight procedure, or a given DAG dependency structure among the
builders shall be properly preserved among their artifacts.

Minimal Rust version: **1.40**



### Basic Concept

The basic concept of daab revolves around _Builders_, which are user provided
structs that implement the [`Builder`] trait. That trait essentially has an
associated type [`Artifact`] and method [`build`] where the latter will
produce a value of the `Artifact` type, which will be subsequently be
referred to as _Artifact_. In order to be able to depend on the Artifact of
other Builders, the `build` method also gets a [`Resolver`] that allows
to retrieve the Artifacts of others.

In order to allow Builders and Artifacts to form a directed acyclic graph
this crate provides at its heart an Artifact [`Cache`] which keeps the
Artifacts of Builders in order to prevent the Builders to produce multiple
equal Artifacts. Thus different Builders may depend on same Builder and
getting the same Artifact from the `Cache`.

To be able to share Builders and Artifacts this crate also provides a
concept of _Cans_ and _Bins_, which in the most basic case are simply an opaque
`Rc<dyn Any>` and a transparent `Rc<T>`, respectively. These are referred to
by the generic arguments of e.g. the `Cache`. For more details consult the
[`canning`] module.

Additional to the canning, the `Cache` expects Builders to wrapped in a
opaque [`Blueprint`] enforcing encapsulation, i.e. it prevents users from
accessing the inner struct (the one which implements the `Builder` trait),
while only allowing the `Cache` itself to call its `build` method.



#### Getting started

For the basic concept (explained above) there exists simplified traits
which skip over the more
advanced features. One such simplified trait is the [`SimpleBuilder`] of the
[`rc`] module, which uses `Rc`s for canning and has simplified aliases
(minimal generic arguments) for all the above types. For getting started
that `rc` module is probably the best place to start.



### Detailed Concept

See the [Advanced Feature section of `Builder`].

Also see [`Cache`], [`Builder`], [`blueprint`], [`canning`]


[`Builder`]: trait.Builder.html
[`Artifact`]: trait.Builder.html#associatedtype.Artifact
[`build`]: trait.Builder.html#tymethod.build
[`SimpleBuilder`]: rc/trait.SimpleBuilder.html
[`rc`]: rc/index.html
[`canning`]: canning/index.html
[`blueprint`]: blueprint/index.html
[`Blueprint`]: blueprint/struct.Blueprint.html
[`Resolver`]: cache/struct.Resolver.html
[`Cache`]: cache/struct.Cache.html
[Advanced Feature section of `Builder`]: trait.Builder.html#advanced-features


### Example

```rust
use std::rc::Rc;
use daab::*;

// Simple artifact
#[derive(Debug)]
struct Leaf {
    //...
}

// Simple builder
#[derive(Debug)]
struct BuilderLeaf {
    // ...
}
impl BuilderLeaf {
    pub fn new() -> Self {
        Self {
            // ...
        }
    }
}
impl rc::SimpleBuilder for BuilderLeaf {
    type Artifact = Leaf;

    fn build(&self, _resolver: &mut rc::Resolver) -> Self::Artifact {
        Leaf{
            // ...
        }
    }
}

// Composed artifact, linking to a Leaf
#[derive(Debug)]
struct Node {
    leaf: Rc<Leaf>, // Dependency artifact
    value: u8, // Some custom value
    // ...
}

// Composed builder, depending on BuilderLeaf
#[derive(Debug)]
struct BuilderNode {
    builder_leaf: rc::Blueprint<BuilderLeaf>, // Dependency builder
    // ...
}
impl BuilderNode {
    pub fn new(builder_leaf: rc::Blueprint<BuilderLeaf>) -> Self {
        Self {
            builder_leaf,
            // ...
        }
    }
}
use std::any::Any;
impl rc::Builder for BuilderNode {
    type Artifact = Node;
    type DynState = u8;
    type Err = Never;

    fn build(&self, resolver: &mut rc::Resolver<Self::DynState>) -> Result<Rc<Self::Artifact>, Never> {
        // Resolve Blueprint to its artifact
        // Unpacking because the Err type is Never.
        let leaf = resolver.resolve(&self.builder_leaf).unpack();

        Ok(Node {
            leaf,
            value: *resolver.my_state(),
            // ...
        }.into())
    }
    fn init_dyn_state(&self) -> Self::DynState {
        42
    }
}

// The cache to storing already created artifacts
let mut cache = rc::Cache::new();

// Constructing builders
let leaf_builder = rc::Blueprint::new(BuilderLeaf::new());

let node_builder_1 = rc::Blueprint::new(BuilderNode::new(leaf_builder.clone()));
let node_builder_2 = rc::Blueprint::new(BuilderNode::new(leaf_builder.clone()));

// Using the cache to access the artifacts from the builders

// The same builder results in same artifact
assert!(Rc::ptr_eq(&cache.get(&node_builder_1).unpack(), &cache.get(&node_builder_1).unpack()));

// Different builders result in different artifacts
assert!( ! Rc::ptr_eq(&cache.get(&node_builder_1).unpack(), &cache.get(&node_builder_2).unpack()));

// Different artifacts may link the same dependent artifact
assert!(Rc::ptr_eq(&cache.get(&node_builder_1).unpack().leaf, &cache.get(&node_builder_2).unpack().leaf));

// Purge builder 2 to ensure the following does not affect it
cache.purge(&node_builder_2);

// Test dynamic state
assert_eq!(cache.get(&node_builder_1).unpack().value, 42);

// Change state
*cache.dyn_state_mut(&node_builder_1) = 127.into();
// Without invalidation, the cached artefact remains unchanged
assert_eq!(cache.dyn_state(&node_builder_1), &127);
// Invalidate node, and ensure it made use of the state
assert_eq!(cache.get(&node_builder_1).unpack().value, 127);

// State of node 2 remains unchanged
assert_eq!(cache.get_dyn_state(&node_builder_2), None);
assert_eq!(cache.get(&node_builder_2).unpack().value, 42);
```



### Debugging

`daab` comes with extensive debugging gear. However, in order to
keep the production impact as low as possible, the debugging facilities
are capsuled behind the **`diagnostics`** feature.

Of course, the debugging feature is for the user of this crate to
debug their graphs. Therefore, it is rather modelled as a
diagnostics feature (hence the name). The diagnosis
is carried out by a [`Doctor`], which is a trait receiving various
internal events in order to record them, print them, or otherwise help
treating the bug.

Care has been taken to keep the **`diagnostics`** feature broadly applicable
as well as keeping the non-`diagnostics` API compatible with the
`diagnostics`-API, meaning that a project not using the
`diagnostics` feature can be easily converted to using
`diagnostics`, usually by just replacing `Cache::new()`
with `Cache::new_with_doctor()`.
In order to store the `Doctor` the `Cache` is generic to a doctor,
which is important on its creation and for storing it by value.
The rest of the time the `Cache` uses `dyn Doctor` as its default
generic argument.
To ease conversion between them, all creatable `Cache`s
(i.e. not `Cache<dyn Doctor>`) implement `DerefMut` to
`&mut Cache<dyn Doctor>` which has all the important methods
implemented.

[`Doctor`]: diagnostics/trait.Doctor.html



### Features

This crate offers the following features:

- **`diagnostics`** enables elaborate graph and cache interaction debugging.
  It adds the `new_with_doctor()` function to the `Cache` and adds
  the `diagnostics` module with the `Doctor` trait definition and some
  default `Doctor`s.

- **`tynm`** enable the optional dependency on the [`tynm`] crate which adds
  functionality to abbreviate type names, which are used by some default
  `Doctor`s, hence it is only useful in connection with the `diagnostics`
  feature.

- **`unsized`** enables better conversion between unsized Builders with
  [`BlueprintUnsized::into_unsized`]. **This feature requires Nightly
  Rust**.

[`tynm`]: https://crates.io/crates/tynm
[`BlueprintUnsized::into_unsized`]: blueprint/struct.BlueprintUnsized.html#method.into_unsized


## License

Licensed under Apache License, Version 2.0 ([LICENSE](LICENSE) or https://www.apache.org/licenses/LICENSE-2.0).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.
