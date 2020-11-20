


use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU32;
use pretty_assertions::{assert_eq, assert_ne};

use super::*;

use crate::Unpacking;

#[cfg(feature = "diagnostics")]
use crate::diagnostics;

// Dummy counter to differentiate instances
static COUNTER: AtomicU32 = AtomicU32::new(0);


#[derive(Debug, PartialEq, Eq)]
struct Leaf {
	id: u32,
}

#[derive(Debug)]
struct BuilderLeaf {
	// empty
}

impl BuilderLeaf {
	pub(crate) fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl SimpleBuilder for BuilderLeaf {
	type Artifact = Leaf;

	fn build(&self, _cache: &mut Resolver) -> Self::Artifact {
		Leaf{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
		}
	}
}


#[derive(Debug, PartialEq, Eq)]
struct SimpleNode {
	id: u32,
	leaf: BinType<Leaf>,
}

#[derive(Debug)]
struct BuilderSimpleNode {
	leaf: Blueprint<BuilderLeaf>,
}

impl BuilderSimpleNode {
	pub(crate) fn new(leaf: Blueprint<BuilderLeaf>) -> Self {
		Self {
			leaf,
		}
	}
}

impl SimpleBuilder for BuilderSimpleNode {
	type Artifact = SimpleNode;

	fn build(&self, cache: &mut Resolver) -> Self::Artifact {
		let leaf = cache.resolve(&self.leaf).unpack();

		SimpleNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			leaf
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
enum LeafOrNodes {
	Leaf(BinType<Leaf>),
	Nodes {
		left: BinType<ComplexNode>,
		right: BinType<ComplexNode>
	},
}

#[derive(Debug)]
enum BuilderLeafOrNodes {
	Leaf(Blueprint<BuilderLeaf>),
	Nodes {
		left: Blueprint<BuilderComplexNode>,
		right: Blueprint<BuilderComplexNode>
	},
}

// Fixes in the Arc case:
// error[E0275]: overflow evaluating the requirement
// `std::sync::Arc<(dyn std::any::Any + std::marker::Send + std::marker::Sync + 'static)>: canning::Can<test_arc::BuilderComplexNode>`
#[allow(unsafe_code)]
unsafe impl Send for BuilderLeafOrNodes {}
#[allow(unsafe_code)]
unsafe impl Sync for BuilderLeafOrNodes {}

impl BuilderLeafOrNodes {
	fn build(&self, cache: &mut Resolver) -> LeafOrNodes {
		match self {
			Self::Leaf(l) => {
				LeafOrNodes::Leaf(cache.resolve(l).unpack())
			},
			Self::Nodes{left, right} => {
				LeafOrNodes::Nodes{
					left: cache.resolve(left).unpack(),
					right: cache.resolve(right).unpack(),
				}
			},
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
struct ComplexNode {
	id: u32,
	inner: LeafOrNodes,
}

impl ComplexNode {
	pub(crate) fn leaf(&self) -> Option<&BinType<Leaf>> {
		if let LeafOrNodes::Leaf(ref l) = self.inner {
			Some(l)
		} else {
			None
		}
	}

	pub(crate) fn left(&self) -> Option<&BinType<ComplexNode>> {
		if let LeafOrNodes::Nodes{ref left, ..} = self.inner {
			Some(left)
		} else {
			None
		}
	}

	pub(crate) fn right(&self) -> Option<&BinType<ComplexNode>> {
		if let LeafOrNodes::Nodes{ref right, ..} = self.inner {
			Some(right)
		} else {
			None
		}
	}
}

#[derive(Debug)]
struct BuilderComplexNode {
	inner: BuilderLeafOrNodes,
}

impl BuilderComplexNode {
	pub(crate) fn new_leaf(leaf: Blueprint<BuilderLeaf>) -> Self {
		Self {
			inner: BuilderLeafOrNodes::Leaf(leaf),
		}
	}

	pub(crate) fn new_nodes(left: Blueprint<BuilderComplexNode>, right: Blueprint<BuilderComplexNode>) -> Self {
		Self {
			inner: BuilderLeafOrNodes::Nodes{left, right},
		}
	}
}

impl SimpleBuilder for BuilderComplexNode {
	type Artifact = ComplexNode;

	fn build(&self, cache: &mut Resolver) -> Self::Artifact {
		ComplexNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			inner: self.inner.build(cache),
		}
	}
}

#[test]
fn test_leaf_broken() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&leaf1), cache.get(&leaf2));
}

#[test]
fn test_dyn_builder() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));

	let artifact_node = cache.get(&noden1);


	let noden1_unsized: DynamicBlueprint<ComplexNode> = noden1.clone().into();

	assert_eq!(artifact_node, cache.get(&noden1_unsized));

	// Try it again
	assert_eq!(artifact_node, cache.get(&noden1_unsized));

}

#[test]
fn test_dyn_builder_complex() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = Blueprint::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = Blueprint::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));

	let artifact_node = cache.get(&noden1);
	let artifact_root = cache.get(&noden3);

	let mut unsized_vec = Vec::new();

	let noden1_unsized: DynamicBlueprint<ComplexNode> = noden1.clone().into();
	assert_eq!(cache.get(&noden1), cache.get(&noden1_unsized));
	unsized_vec.push(noden1_unsized);

	let noden2_unsized: DynamicBlueprint<ComplexNode> = noden2.clone().into();
	assert_eq!(cache.get(&noden2), cache.get(&noden2_unsized));
	unsized_vec.push(noden2_unsized);

	let noden3_unsized: DynamicBlueprint<ComplexNode> = noden3.clone().into();
	assert_eq!(cache.get(&noden3), cache.get(&noden3_unsized));
	unsized_vec.push(noden3_unsized);


	let artifact_vec: Vec<_> = unsized_vec.iter().map( |ap|
		cache.get(ap)
	).collect();

	unsized_vec.iter().zip(artifact_vec.into_iter()).for_each( |(ap,art)| {
		assert_eq!(
			cache.get(ap),
			art
		);
	});

}




