


use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU32;
use pretty_assertions::{assert_eq, assert_ne};

use std::marker::PhantomData;

use crate::*;

// Dummy counter to differentiate instances
static COUNTER: AtomicU32 = AtomicU32::new(0);


#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Leaf {
	id: u32,
}

#[derive(Debug)]
pub(crate) struct BuilderLeaf {
	// empty
}

impl BuilderLeaf {
	pub(crate) fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl<ArtCan,BCan> Builder<ArtCan,BCan> for BuilderLeaf
	where
		ArtCan: CanSized<Leaf>,
		BCan: CanStrong {

	type Artifact = Leaf;

	type DynState = ();

	type Err = Never;

	fn build(&self, _cache: &mut Resolver<ArtCan,BCan>) -> Result<ArtCan::Bin, Never> {
		Ok(ArtCan::into_bin(Leaf{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
		}))
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}


#[derive(Debug)]
pub(crate) struct BuilderLeafFallible {
	// empty
}

impl BuilderLeafFallible {
	pub(crate) fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl<ArtCan,BCan> Builder<ArtCan,BCan> for BuilderLeafFallible
	where
		ArtCan: Debug,
		ArtCan: CanSized<Leaf>,
		BCan: CanStrong {

	type Artifact = Leaf;

	type DynState = bool;

	type Err = ();

	fn build(&self, cache: &mut Resolver<ArtCan,BCan,bool>) -> Result<ArtCan::Bin, ()> {
		if *cache.my_state() {
			Ok(ArtCan::into_bin(Leaf{
				id: COUNTER.fetch_add(1, Ordering::SeqCst),
			}))
		} else {
			Err(())
		}
	}
	fn init_dyn_state(&self) -> Self::DynState {
		true
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SimpleNode<Bin> {
	id: u32,
	leaf: Bin,
}

#[derive(Debug)]
pub(crate) struct BuilderSimpleNode<AP> {
	leaf: AP,
}

impl<AP> BuilderSimpleNode<AP> {

	pub(crate) fn new<BCan: Debug>(leaf: AP) -> Self
		where
			AP: Promise<BuilderLeaf, BCan>,
			BCan: Can<BuilderLeaf>, {

		Self {
			leaf,
		}
	}
}

impl<AP, ArtCan: Debug, BCan> Builder<ArtCan, BCan> for BuilderSimpleNode<AP>
	where
		AP: Promise<BuilderLeaf, BCan> + Debug,
		ArtCan: Clone,
		ArtCan: CanSized<Leaf>,
		ArtCan: CanSized<SimpleNode<<ArtCan as Can<Leaf>>::Bin>>,
		BCan: CanStrong,
		{

	type Artifact = SimpleNode<<ArtCan as Can<Leaf>>::Bin>;

	type DynState = ();

	type Err = Never;

	fn build(&self, cache: &mut Resolver<ArtCan,BCan>)
		-> Result<<ArtCan as Can<SimpleNode<<ArtCan as Can<Leaf>>::Bin>>>::Bin, Never> {

		let leaf = cache.resolve(&self.leaf)?;

		Ok(ArtCan::into_bin(SimpleNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			leaf
		}))
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}

#[derive(Debug)]
pub(crate) struct BuilderVariableNode<B,AP> {
	leaf: AP,
	_b: PhantomData<B>,
}

impl<B, AP> BuilderVariableNode<B, AP> {

	pub(crate) fn new<ArtCan, BCan: Debug>(leaf: AP) -> Self
		where
			B: Builder<ArtCan, BCan>,
			B::Err: Into<()>,
			AP: Promise<B, BCan> + Clone,
			ArtCan: Clone,
			ArtCan: CanSized<B::Artifact>,
			BCan: CanStrong, {

		Self {
			leaf,
			_b: PhantomData,
		}
	}
}

impl<B, AP, ArtCan, BCan> Builder<ArtCan, BCan> for BuilderVariableNode<B, AP>
	where
		B: Builder<ArtCan, BCan>,
		(): From<B::Err>, //aka, B::Err: Into<()>,
		AP: Promise<B, BCan> + Clone,
		ArtCan: Clone,
		ArtCan: CanSized<B::Artifact>,
		ArtCan: CanSized<SimpleNode<<ArtCan as Can<B::Artifact>>::Bin>>,
		BCan: CanStrong,
		{

	type Artifact = SimpleNode<<ArtCan as Can<B::Artifact>>::Bin>;

	type DynState = (AP, bool);

	type Err = ();

	fn build(&self, cache: &mut Resolver<ArtCan,BCan,(AP,bool)>)
		-> Result<<ArtCan as Can<SimpleNode<<ArtCan as Can<B::Artifact>>::Bin>>>::Bin, ()> {

		let dyn_ap = cache.my_state().0.clone();
		let leaf = cache.resolve(&dyn_ap)?;

		if cache.my_state().1 {
			Ok(ArtCan::into_bin(SimpleNode{
				id: COUNTER.fetch_add(1, Ordering::SeqCst),
				leaf
			}))
		} else {
			Err(())
		}
	}
	fn init_dyn_state(&self) -> Self::DynState {
		(
			self.leaf.clone(),
			true,
		)
	}
}


#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ComplexNode<L,R> {
	id: u32,
	left: L,
	right: R,
}


#[derive(Debug)]
pub(crate) struct BuilderComplexNode<ApL,ApR,LB,RB> {
	left: ApL,
	right: ApR,
	_l_t: PhantomData<LB>,
	_r_t: PhantomData<RB>,
}

impl<ApL,ApR,LB,RB> BuilderComplexNode<ApL,ApR,LB,RB> {

	pub(crate) fn new(
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

type ComplexNodeByAp<ArtCan,BCan,LB,RB> = ComplexNode<
	<ArtCan as Can<<LB as Builder<ArtCan,BCan>>::Artifact>>::Bin,
	<ArtCan as Can<<RB as Builder<ArtCan,BCan>>::Artifact>>::Bin,
>;

impl<ApL, ApR, ArtCan: Debug, BCan: Debug, LB, RB> Builder<ArtCan, BCan> for BuilderComplexNode<ApL, ApR, LB, RB>
	where
		LB: LeafOrNodeBuilder<ArtCan, BCan> + 'static,
		RB: LeafOrNodeBuilder<ArtCan, BCan> + 'static,
		ApL: Promise<LB, BCan>,
		ApR: Promise<RB, BCan>,
		ArtCan: Clone,
		ArtCan: CanSized<LB::Artifact>,
		ArtCan: CanSized<RB::Artifact>,
		ArtCan: CanSized<ComplexNodeByAp<ArtCan, BCan, LB, RB>>,
		BCan: CanStrong,
		{

	/*
	type Artifact = ComplexNode<
		<ArtCan as Can<LB::Artifact>>::Bin,
		<ArtCan as Can<RB::Artifact>>::Bin,
	>;
	*/
	type Artifact = ComplexNodeByAp<ArtCan, BCan, LB, RB>;

	type DynState = ();

	type Err = Never;

	fn build(&self, cache: &mut Resolver<ArtCan, BCan>) -> Result<<ArtCan as Can<ComplexNodeByAp<ArtCan, BCan, LB, RB>>>::Bin, Never> {
		Ok(ArtCan::into_bin(ComplexNode {
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			left: cache.resolve(&self.left)?,
			right: cache.resolve(&self.right)?,
		}))
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}

pub(crate) trait LeafOrNodeBuilder<ArtCan, BCan>: Builder<ArtCan, BCan, Err=Never> where BCan: CanStrong {}

impl<AP, ArtCan, BCan> LeafOrNodeBuilder<ArtCan, BCan> for BuilderSimpleNode<AP>
	where
		AP: Promise<BuilderLeaf, BCan> + Debug,
		ArtCan: Clone,
		ArtCan: CanSized<Leaf>,
		ArtCan: CanSized<SimpleNode<<ArtCan as Can<Leaf>>::Bin>>,
		BCan: CanStrong,
	{

}

impl<ApL, ApR, LB, RB, ArtCan, BCan> LeafOrNodeBuilder<ArtCan, BCan> for BuilderComplexNode<ApL, ApR, LB, RB>
	where
		LB: LeafOrNodeBuilder<ArtCan, BCan> + 'static,
		RB: LeafOrNodeBuilder<ArtCan, BCan> + 'static,
		ApL: Promise<LB, BCan>,
		ApR: Promise<RB, BCan>,
		ArtCan: Clone,
		ArtCan: CanSized<LB::Artifact>,
		ArtCan: CanSized<RB::Artifact>,
		ArtCan: CanSized<ComplexNodeByAp<ArtCan, BCan, LB, RB>>,
		BCan: CanStrong,
	{

}



#[derive(Debug)]
pub(crate) struct BuilderLeafBox {
	// empty
}

impl BuilderLeafBox {
	pub(crate) fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl crate::boxed::Builder for BuilderLeafBox {
	type Artifact = Leaf;
	type DynState = ();
	type Err = Never;

	fn build(&self, _cache: &mut crate::boxed::Resolver) -> Result<Box<Self::Artifact>, Never> {
		Ok(Box::new(Leaf{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
		}))
	}
	fn init_dyn_state(&self) -> Self::DynState {
		// empty
	}
}



fn as_ptr<T,E>(res: Result<&T,E>) -> Result<*const T,E> {
	res.map(|v| v as *const T)
}

fn as_ptr_mut<T,E>(res: Result<&mut T,E>) -> Result<*mut T,E> {
	res.map(|v| v as *mut T)
}

#[test]
fn test_boxed_ref() {
	let mut cache = crate::boxed::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeafBox::new());
	let leaf2 = Blueprint::new(BuilderLeafBox::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(as_ptr(cache.get_ref(&leaf1)), as_ptr(cache.get_ref(&leaf1)));

	// Ensure different builder result in different artifacts
	assert_ne!(as_ptr(cache.get_ref(&leaf1)), as_ptr(cache.get_ref(&leaf2)));
}

#[test]
fn test_boxed_mut() {
	let mut cache = crate::boxed::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeafBox::new());
	let leaf2 = Blueprint::new(BuilderLeafBox::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(as_ptr_mut(cache.get_mut(&leaf1)), as_ptr_mut(cache.get_mut(&leaf1)));

	// Ensure different builder result in different artifacts
	assert_ne!(as_ptr_mut(cache.get_mut(&leaf1)), as_ptr_mut(cache.get_mut(&leaf2)));
}

// Tests whether it is valid to get a Cache by &mut
fn ref_function<Art, B, P>(cache: &mut crate::Cache<Art, B>, l: Art::Bin, ap: &P)
	where
		Art: CanSized<Leaf> + Clone,
		B: CanSized<BuilderLeaf> + CanStrong,
		P: Promise<BuilderLeaf, B>,
		Art::Bin: PartialEq, {

	assert_eq!(cache.get(ap).unwrap(), l);
}


#[test]
fn test_ref_function() {
	let mut cache = crate::rc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&leaf1), cache.get(&leaf2));

	// Ensure sub routies get the same instances
	let parent = cache.get(&leaf1);

	ref_function(&mut cache, parent.unpack(), &leaf1);
}


#[test]
fn test_leaf_broken() {
	use std::rc::Rc;

	let mut cache_rc: CacheOwned<Rc<dyn Any>, Rc<dyn Any>> = Cache::new();
	let mut cache_box = CacheOwned::<Box<dyn Any>, Rc<dyn Any>>::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache_rc.get(&leaf1), cache_rc.get(&leaf1));
	assert_eq!(cache_rc.get_cloned(&leaf1), cache_rc.get_cloned(&leaf1));
	assert_eq!(*cache_rc.get(&leaf1).unpack(), *cache_rc.get_ref(&leaf1).unpack());

	// Ensure different builder result in different artifacts
	assert_ne!(*cache_rc.get(&leaf1).unpack(), *cache_rc.get_ref(&leaf2).unpack());
	assert_ne!(*cache_rc.get(&leaf1).unpack(), *cache_rc.get_ref(&leaf2).unpack());

	// Ensure same builder results in same artifact
	assert_eq!(cache_box.get_cloned(&leaf1), cache_box.get_cloned(&leaf1));
	assert_eq!(cache_box.get_cloned(&leaf1).unpack(), *cache_box.get_ref(&leaf1).unpack());

	// Ensure different builder result in different artifacts
	assert_ne!(cache_box.get_cloned(&leaf1).unpack(), *cache_box.get_ref(&leaf2).unpack());

	// Ensure different builder result in different artifacts
	assert_ne!(*cache_rc.get_ref(&leaf1).unpack(), *cache_box.get_ref(&leaf1).unpack());
}


#[test]
fn test_leaf_rc() {
	let mut cache = rc::Cache::new();

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
fn test_leaf_boxed() {
	let mut cache = boxed::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id(), leaf2.id());

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_cloned(&leaf1), cache.get_cloned(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&leaf1), cache.get_cloned(&leaf2));
}

#[test]
fn test_node_rc() {
	let mut cache = rc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));
	let node3 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&node1), cache.get(&node1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&node2), cache.get(&node3));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get(&node2).unpack().leaf, cache.get(&node3).unpack().leaf);

}

#[test]
fn test_node_arc() {
	let mut cache = arc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));
	let node3 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&node1), cache.get(&node1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&node2), cache.get(&node3));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get(&node2).unpack().leaf, cache.get(&node3).unpack().leaf);

}

/*
#[test]
fn test_node_boxed() {
	use std::rc::Rc;

	let mut cache = CacheOwned::<Box<dyn Any>, Rc<dyn Any>>::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));
	let node3 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));

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
	let mut cache = rc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));
	let nodef3 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new(
		nodef1.clone(),
		nodef2.clone()
	));
	let noden2 = Blueprint::new(BuilderComplexNode::new(
		nodef3.clone(),
		noden1.clone()
	));
	let noden3 = Blueprint::new(BuilderComplexNode::new(
		noden2.clone(),
		noden2.clone()
	));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&noden3), cache.get(&noden3));

	// Ensure different builder result in different artifacts
	//assert_ne!(cache.get(&noden1), cache.get(&noden2));

	let artifact_leaf = cache.get(&leaf1).unpack();
	let artifact_node = cache.get(&noden1).unpack();
	let artifact_root = cache.get(&noden3).unpack();

	assert_eq!(artifact_root.left, artifact_root.right);

	assert_eq!(artifact_root.left.right, artifact_node);
	assert_eq!(artifact_node.left.leaf, artifact_leaf);

}

#[test]
fn test_complex_clear() {
	let mut cache = rc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));
	let nodef3 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new(
		nodef1.clone(),
		nodef2.clone()
	));
	let noden2 = Blueprint::new(BuilderComplexNode::new(
		nodef3.clone(),
		noden1.clone()
	));
	let noden3 = Blueprint::new(BuilderComplexNode::new(
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
	let mut cache = rc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());

	let artifact1 = cache.get(&leaf1);

	cache.invalidate(&leaf1);

	let artifact2 = cache.get(&leaf1);

	// Ensure artifacts differ after clear
	assert_ne!(artifact1, artifact2);

}

#[test]
fn test_into() {
	let mut cache = rc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let lart = cache.get(&leaf1).unpack();

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1));

	assert_eq!(cache.get(&node1).unpack().leaf.as_ref(), lart.as_ref());
}

#[test]
fn test_complex_invalidate() {
	let mut cache = rc::Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));
	let nodef3 = Blueprint::new(BuilderSimpleNode::new(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new(
		nodef1.clone(),
		nodef2.clone()
	));
	let noden2 = Blueprint::new(BuilderComplexNode::new(
		nodef3.clone(),
		noden1.clone()
	));
	let noden3 = Blueprint::new(BuilderComplexNode::new(
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

