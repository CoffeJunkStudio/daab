


use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU32;
use pretty_assertions::{assert_eq, assert_ne};

use std::marker::PhantomData;

use crate::*;

// Dummy counter to differentiate instances
static COUNTER: AtomicU32 = AtomicU32::new(0);


#[derive(Debug, Clone, PartialEq, Eq)]
struct Leaf {
	id: u32,
}

#[derive(Debug)]
struct BuilderLeaf {
	// empty
}

impl BuilderLeaf {
	pub fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl<ArtCan,BCan> Builder<ArtCan,BCan> for BuilderLeaf
	where
		BCan: CanStrong {

	type Artifact = Leaf;

	type DynState = ();

	fn build(&self, _cache: &mut ArtifactResolver<ArtCan,BCan>) -> Self::Artifact {
		Leaf{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
		}
	}
}


#[derive(Debug, Clone, PartialEq, Eq)]
struct SimpleNode<Bin> {
	id: u32,
	leaf: Bin,
}

#[derive(Debug)]
struct BuilderSimpleNode<AP> {
	leaf: AP,
}

impl<AP> BuilderSimpleNode<AP> {

	pub fn new<BCan: Debug>(leaf: AP) -> Self
		where
			AP: ArtifactPromiseTrait<BuilderLeaf, BCan>,
			BCan: Can<BuilderLeaf>, {

		Self {
			leaf,
		}
	}
}

impl<AP, ArtCan: Debug, BCan: Debug> Builder<ArtCan, BCan> for BuilderSimpleNode<AP>
	where
		AP: ArtifactPromiseTrait<BuilderLeaf, BCan> + Debug,
		ArtCan: Clone,
		ArtCan: CanSized<Leaf>,
		ArtCan::Bin: 'static,
		BCan: CanStrong,
		{

	type Artifact = SimpleNode<ArtCan::Bin>;

	type DynState = ();

	fn build(&self, cache: &mut ArtifactResolver<ArtCan,BCan>)
		-> Self::Artifact {

		let leaf = cache.resolve(&self.leaf);

		SimpleNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			leaf
		}
	}
}


#[derive(Debug, PartialEq, Eq)]
struct ComplexNode<L,R> {
	id: u32,
	left: L,
	right: R,
}


#[derive(Debug)]
struct BuilderComplexNode<ApL,ApR,LB,RB> {
	left: ApL,
	right: ApR,
	_l_t: PhantomData<LB>,
	_r_t: PhantomData<RB>,
}

impl<ApL,ApR,LB,RB> BuilderComplexNode<ApL,ApR,LB,RB> {

	pub fn new(
		left: ApL,
		right: ApR,
	) -> Self {
		BuilderComplexNode {
			left,
			right,
			_l_t: PhantomData,
			_r_t: PhantomData,
		}
	}
}

impl<ApL, ApR, ArtCan: Debug, BCan: Debug, LB: 'static, RB: 'static> Builder<ArtCan, BCan> for BuilderComplexNode<ApL, ApR, LB, RB>
	where
		LB: LeafOrNodeBuilder<ArtCan, BCan>,
		RB: LeafOrNodeBuilder<ArtCan, BCan>,
		ApL: ArtifactPromiseTrait<LB, BCan> + Debug,
		ApR: ArtifactPromiseTrait<RB, BCan> + Debug,
		ArtCan: Clone,
		ArtCan: CanSized<<LB as Builder<ArtCan, BCan>>::Artifact>,
		ArtCan: CanSized<<RB as Builder<ArtCan, BCan>>::Artifact>,
		<ArtCan as Can<<LB as Builder<ArtCan, BCan>>::Artifact>>::Bin: 'static,
		<ArtCan as Can<<RB as Builder<ArtCan, BCan>>::Artifact>>::Bin: 'static,
		BCan: CanStrong,
		{

	type Artifact = ComplexNode<
		<ArtCan as Can<<LB as Builder<ArtCan, BCan>>::Artifact>>::Bin,
		<ArtCan as Can<<RB as Builder<ArtCan, BCan>>::Artifact>>::Bin,
	>;

	type DynState = ();

	fn build(&self, cache: &mut ArtifactResolver<ArtCan, BCan>) -> Self::Artifact {
		ComplexNode {
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			left: cache.resolve(&self.left),
			right: cache.resolve(&self.right),
		}
	}
}

//trait LeafOrNode<ArtCan, BCan>: Debug + Builder<ArtCan, BCan> where BCan: CanStrong {}

trait LeafOrNodeBuilder<ArtCan, BCan>: Debug + Builder<ArtCan, BCan> where BCan: CanStrong {}

impl<AP, ArtCan, BCan> LeafOrNodeBuilder<ArtCan, BCan> for BuilderSimpleNode<AP>
	where
		AP: ArtifactPromiseTrait<BuilderLeaf, BCan> + Debug,
		ArtCan: Clone + Debug,
		ArtCan: CanSized<Leaf>,
		ArtCan::Bin: 'static,
		BCan: Debug + CanStrong,
	{

}

impl<ApL, ApR, LB, RB, ArtCan, BCan> LeafOrNodeBuilder<ArtCan, BCan> for BuilderComplexNode<ApL, ApR, LB, RB>
	where
		LB: LeafOrNodeBuilder<ArtCan, BCan> + 'static,
		RB: LeafOrNodeBuilder<ArtCan, BCan> + 'static,
		ApL: ArtifactPromiseTrait<LB, BCan> + Debug,
		ApR: ArtifactPromiseTrait<RB, BCan> + Debug,
		ArtCan: Clone + Debug,
		ArtCan: CanSized<<LB as Builder<ArtCan, BCan>>::Artifact>,
		ArtCan: CanSized<<RB as Builder<ArtCan, BCan>>::Artifact>,
		<ArtCan as Can<<LB as Builder<ArtCan, BCan>>::Artifact>>::Bin: 'static,
		<ArtCan as Can<<RB as Builder<ArtCan, BCan>>::Artifact>>::Bin: 'static,
		BCan: Debug + CanStrong,
	{

}



#[derive(Debug)]
struct BuilderLeafBox {
	// empty
}

impl BuilderLeafBox {
	pub fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl crate::boxed::Builder for BuilderLeafBox {
	type Artifact = Leaf;
	type DynState = ();

	fn build(&self, _cache: &mut crate::boxed::ArtifactResolver) -> Self::Artifact {
		Leaf{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
		}
	}
}




#[test]
fn test_boxed_ref() {
	let mut cache = crate::boxed::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeafBox::new());
	let leaf2 = ArtifactPromise::new(BuilderLeafBox::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_ref(&leaf1) as *const Leaf, cache.get_ref(&leaf1) as *const Leaf);

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_ref(&leaf1) as *const Leaf, cache.get_ref(&leaf2) as *const Leaf);
}

#[test]
fn test_boxed_mut() {
	let mut cache = crate::boxed::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeafBox::new());
	let leaf2 = ArtifactPromise::new(BuilderLeafBox::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_mut(&leaf1) as *const Leaf, cache.get_ref(&leaf1) as *const Leaf);

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_mut(&leaf1) as *mut Leaf, cache.get_mut(&leaf2) as *mut Leaf);
}

#[test]
fn test_leaf_broken() {
	use std::rc::Rc;

	let mut cache_rc: ArtifactCacheOwned<Rc<dyn Any>, Rc<dyn Any>> = ArtifactCache::new();
	let mut cache_box = ArtifactCacheOwned::<Box<dyn Any>, Rc<dyn Any>>::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache_rc.get(&leaf1), cache_rc.get(&leaf1));
	assert_eq!(cache_rc.get_cloned(&leaf1), cache_rc.get_cloned(&leaf1));
	assert_eq!(*cache_rc.get(&leaf1), *cache_rc.get_ref(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(*cache_rc.get(&leaf1), *cache_rc.get_ref(&leaf2));
	assert_ne!(*cache_rc.get(&leaf1), *cache_rc.get_ref(&leaf2));

	// Ensure same builder results in same artifact
	assert_eq!(cache_box.get_cloned(&leaf1), cache_box.get_cloned(&leaf1));
	assert_eq!(cache_box.get_cloned(&leaf1), *cache_box.get_ref(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache_box.get_cloned(&leaf1), *cache_box.get_ref(&leaf2));

	// Ensure different builder result in different artifacts
	assert_ne!(*cache_rc.get_ref(&leaf1), *cache_box.get_ref(&leaf1));
}


#[test]
fn test_leaf_rc() {
	let mut cache = rc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&leaf1), cache.get(&leaf2));
}

#[test]
fn test_leaf_boxed() {
	let mut cache = boxed::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_cloned(&leaf1), cache.get_cloned(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&leaf1), cache.get_cloned(&leaf2));
}

#[test]
fn test_node_rc() {
	let mut cache = rc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));
	let node3 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&node1), cache.get(&node1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&node2), cache.get(&node3));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get(&node2).leaf, cache.get(&node3).leaf);

}

#[test]
fn test_node_arc() {
	let mut cache = arc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));
	let node3 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&node1), cache.get(&node1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&node2), cache.get(&node3));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get(&node2).leaf, cache.get(&node3).leaf);

}

/*
#[test]
fn test_node_boxed() {
	use std::rc::Rc;

	let mut cache = ArtifactCacheOwned::<Box<dyn Any>, Rc<dyn Any>>::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));
	let node3 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_cloned(&node1), cache.get_cloned(&node1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&node2), cache.get_cloned(&node3));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get_cloned(&node2).leaf, cache.get_cloned(&node3).leaf);

}
*/

#[test]
fn test_complex() {
	let mut cache = rc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	let nodef1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let nodef2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));
	let nodef3 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));

	let noden1 = ArtifactPromise::new(BuilderComplexNode::new(
		nodef1.clone(),
		nodef2.clone()
	));
	let noden2 = ArtifactPromise::new(BuilderComplexNode::new(
		nodef3.clone(),
		noden1.clone()
	));
	let noden3 = ArtifactPromise::new(BuilderComplexNode::new(
		noden2.clone(),
		noden2.clone()
	));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&noden3), cache.get(&noden3));

	// Ensure different builder result in different artifacts
	//assert_ne!(cache.get(&noden1), cache.get(&noden2));

	let artifact_leaf = cache.get(&leaf1);
	let artifact_node = cache.get(&noden1);
	let artifact_root = cache.get(&noden3);

	assert_eq!(artifact_root.left, artifact_root.right);

	assert_eq!(artifact_root.left.right, artifact_node);
	assert_eq!(artifact_node.left.leaf, artifact_leaf);

}

#[test]
fn test_complex_clear() {
	let mut cache = rc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	let nodef1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let nodef2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));
	let nodef3 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));

	let noden1 = ArtifactPromise::new(BuilderComplexNode::new(
		nodef1.clone(),
		nodef2.clone()
	));
	let noden2 = ArtifactPromise::new(BuilderComplexNode::new(
		nodef3.clone(),
		noden1.clone()
	));
	let noden3 = ArtifactPromise::new(BuilderComplexNode::new(
		noden2.clone(),
		noden2.clone()
	));

	let artifact_leaf = cache.get(&leaf1);
	let artifact_node = cache.get(&noden1);
	let artifact_root = cache.get(&noden3);

	cache.clear_all();

	let artifact_root_2 = cache.get(&noden3);
	let artifact_node_2 = cache.get(&noden1);
	let artifact_leaf_2 = cache.get(&leaf1);

	assert_ne!(artifact_leaf, artifact_leaf_2);
	assert_ne!(artifact_node, artifact_node_2);
	assert_ne!(artifact_root, artifact_root_2);

}

#[test]
fn test_invalidate() {
	let mut cache = rc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());

	let artifact1 = cache.get(&leaf1);

	cache.invalidate(&leaf1);

	let artifact2 = cache.get(&leaf1);

	// Ensure artifacts differ after clear
	assert_ne!(artifact1, artifact2);

}

#[test]
fn test_into() {
	let mut cache = rc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let lart = cache.get(&leaf1);

	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1));

	assert_eq!(cache.get(&node1).leaf.as_ref(), lart.as_ref());
}

#[test]
fn test_complex_invalidate() {
	let mut cache = rc::ArtifactCache::new();

	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());

	let nodef1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let nodef2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));
	let nodef3 = ArtifactPromise::new(BuilderSimpleNode::new(leaf2.clone()));

	let noden1 = ArtifactPromise::new(BuilderComplexNode::new(
		nodef1.clone(),
		nodef2.clone()
	));
	let noden2 = ArtifactPromise::new(BuilderComplexNode::new(
		nodef3.clone(),
		noden1.clone()
	));
	let noden3 = ArtifactPromise::new(BuilderComplexNode::new(
		noden2.clone(),
		noden2.clone()
	));

	let artifact_leaf = cache.get(&leaf1);
	let artifact_node = cache.get(&noden1);
	let artifact_root = cache.get(&noden3);

	// Only invalidate one intermediate node
	cache.invalidate(&noden1);

	let artifact_leaf_2 = cache.get(&leaf1);
	let artifact_node_2 = cache.get(&noden1);
	let artifact_root_2 = cache.get(&noden3);

	assert_eq!(artifact_leaf, artifact_leaf_2);
	assert_ne!(artifact_node, artifact_node_2);
	assert_ne!(artifact_root, artifact_root_2);

}

