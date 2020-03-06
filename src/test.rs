use super::*;

use std::rc::Rc;
use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU32;
use pretty_assertions::{assert_eq, assert_ne};


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
	pub fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl SimpleBuilder for BuilderLeaf {
	type Artifact = Leaf;
	
	fn build(&self, _cache: &mut ArtifactResolverRc) -> Self::Artifact {
		Leaf{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
		}
	}
}


#[derive(Debug, PartialEq, Eq)]
struct SimpleNode {
	id: u32,
	leaf: Rc<Leaf>,
}

#[derive(Debug)]
struct BuilderSimpleNode {
	leaf: ArtifactPromiseRc<BuilderLeaf>,
}

impl BuilderSimpleNode {
	pub fn new(leaf: ArtifactPromiseRc<BuilderLeaf>) -> Self {
		Self {
			leaf,
		}
	}
}

impl SimpleBuilder for BuilderSimpleNode {
	type Artifact = SimpleNode;
	
	fn build(&self, cache: &mut ArtifactResolverRc) -> Self::Artifact {
		let leaf = cache.resolve(&self.leaf);
		
		SimpleNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			leaf
		}
	}
}

#[derive(Debug, PartialEq, Eq)]
enum LeafOrNodes {
	Leaf(Rc<Leaf>),
	Nodes {
		left: Rc<ComplexNode>,
		right: Rc<ComplexNode>
	},
}

#[derive(Debug)]
enum BuilderLeafOrNodes {
	Leaf(ArtifactPromiseRc<BuilderLeaf>),
	Nodes {
		left: ArtifactPromiseRc<BuilderComplexNode>,
		right: ArtifactPromiseRc<BuilderComplexNode>
	},
}

impl BuilderLeafOrNodes {
	fn build(&self, cache: &mut ArtifactResolverRc) -> LeafOrNodes {
		match self {
			Self::Leaf(l) => {
				LeafOrNodes::Leaf(cache.resolve(l))
			},
			Self::Nodes{left, right} => {
				LeafOrNodes::Nodes{
					left: cache.resolve(left),
					right: cache.resolve(right),
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
	pub fn leaf(&self) -> Option<&Rc<Leaf>> {
		if let LeafOrNodes::Leaf(ref l) = self.inner {
			Some(l)
		} else {
			None
		}
	}
	
	pub fn left(&self) -> Option<&Rc<ComplexNode>> {
		if let LeafOrNodes::Nodes{ref left, ..} = self.inner {
			Some(left)
		} else {
			None
		}
	}
	
	pub fn right(&self) -> Option<&Rc<ComplexNode>> {
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
	pub fn new_leaf(leaf: ArtifactPromiseRc<BuilderLeaf>) -> Self {
		Self {
			inner: BuilderLeafOrNodes::Leaf(leaf),
		}
	}
	
	pub fn new_nodes(left: ArtifactPromiseRc<BuilderComplexNode>, right: ArtifactPromiseRc<BuilderComplexNode>) -> Self {
		Self {
			inner: BuilderLeafOrNodes::Nodes{left, right},
		}
	}
}

impl SimpleBuilder for BuilderComplexNode {
	type Artifact = ComplexNode;
	
	fn build(&self, cache: &mut ArtifactResolverRc) -> Self::Artifact {
		ComplexNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			inner: self.inner.build(cache),
		}
	}
}

#[test]
fn test_leaf_broken() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());
		
	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id, leaf2.id);
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&leaf1), cache.get(&leaf2));
}

#[test]
fn test_leaf() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromiseRc::new(BuilderLeaf::new());
		
	println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	println!("Ptr: {:?}; {:?}", leaf1.id, leaf2.id);
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&leaf1), cache.get(&leaf2));
}

#[test]
fn test_node() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let node1 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf2.clone()));
	let node3 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf2.clone()));
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&node1), cache.get(&node1));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&node2), cache.get(&node3));
	
	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get(&node2).leaf, cache.get(&node3).leaf);
	
}

#[test]
fn test_complex() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let nodef1 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	
	let noden1 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&noden3), cache.get(&noden3));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&noden1), cache.get(&noden2));
	
	let artifact_leaf = cache.get(&leaf1);
	let artifact_node = cache.get(&noden1);
	let artifact_root = cache.get(&noden3);
	
	assert_eq!(artifact_root.left(), artifact_root.right());
	
	assert_eq!(artifact_root.left().unwrap().right(), Some(&artifact_node));
	assert_eq!(artifact_node.left().unwrap().leaf(), Some(&artifact_leaf));
	
}

#[test]
fn test_clear() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let artifact1 = cache.get(&leaf1);
	
	cache.clear();
	
	let artifact2 = cache.get(&leaf1);
	
	// Ensure artifacts differ after clear
	assert_ne!(artifact1, artifact2);
	
}

#[cfg(feature = "diagnostics")]
fn visgraph_doc(buf: Vec<u8>) -> diagnostics::VisgraphDoc<std::io::Cursor<Vec<u8>>> {
	diagnostics::VisgraphDoc::new(
		diagnostics::VisgraphDocOptions {
			show_builder_values: false,
			show_artifact_values: true,
		},
		std::io::Cursor::new(buf),
	)
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_vis_doc() {
	
	// Expected value as Regular Expression due to variable addresses and counters
	let regex = regex::Regex::new(
		r##"strict digraph \{ graph \[labeljust = l\];
  "0x[0-9a-f]+" \[label = "daab::test::BuilderSimpleNode"\]
  "0x[0-9a-f]+" \[label = "daab::test::BuilderLeaf"\]
  "0x[0-9a-f]+" -> "0x[0-9a-f]+"
  "0x[0-9a-f]+" \[label = "daab::test::BuilderLeaf"\]
  "0\.0-0x[0-9a-f]+" \[label = "#0\.0 daab::test::Leaf :
Leaf \{
    id: [0-9]+,
\}", shape = box\]
  "0x[0-9a-f]+" -> "0.0-0x[0-9a-f]+" \[arrowhead = "none"\]
  "0x[0-9a-f]+" \[label = "daab::test::BuilderSimpleNode"\]
  "0\.1-0x[0-9a-f]+" \[label = "#0\.1 daab::test::SimpleNode :
SimpleNode \{
    id: [0-9]+,
    leaf: Leaf \{
        id: [0-9]+,
    \},
\}", shape = box\]
  "0x[0-9a-f]+" -> "0.1-0x[0-9a-f]+" \[arrowhead = "none"\]
  "0x[0-9a-f]+" \[label = "daab::test::BuilderSimpleNode"\]
  "0x[0-9a-f]+" \[label = "daab::test::BuilderLeaf"\]
  "0x[0-9a-f]+" -> "0x[0-9a-f]+"
  "0x[0-9a-f]+" \[label = "daab::test::BuilderSimpleNode"\]
  "0\.2-0x[0-9a-f]+" \[label = "#0\.2 daab::test::SimpleNode :
SimpleNode \{
    id: [0-9]+,
    leaf: Leaf \{
        id: [0-9]+,
    \},
\}", shape = box\]
  "0x[0-9a-f]+" -> "0.2-0x[0-9a-f]+" \[arrowhead = "none"\]
\}"##).unwrap();


	// Visgraph output storage
	let mut data = Vec::new();
	
	
	let mut cache = ArtifactCache::new_with_doctor(visgraph_doc(data));
	
	// Test data
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let node1 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf1.clone()));
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&node2), cache.get(&node2));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&node1), cache.get(&node2));
	
	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get::<BuilderSimpleNode>(&node2).leaf, cache.get(&node1).leaf);
	
	// Get the vector back, dissolves cache & doctor
	data = cache.into_doctor().into_inner().into_inner();
	
	let string = String::from_utf8(data).unwrap();
	// Print the resulting string, very usable in case it does not match
	println!("{}", string);
	
	assert!(regex.is_match(&string));
}



#[test]
#[cfg(feature = "diagnostics")]
fn test_text_doc() {
	
	// Expected value as Regular Expression due to variable addresses and counters
	#[cfg(not(feature = "tynm"))]
	let pattern = r"resolves daab::test::BuilderSimpleNode -> daab::test::BuilderLeaf
built #0.0  daab::test::BuilderLeaf => daab::test::Leaf
built #0.1  daab::test::BuilderSimpleNode => daab::test::SimpleNode
resolves daab::test::BuilderSimpleNode -> daab::test::BuilderLeaf
built #0.2  daab::test::BuilderSimpleNode => daab::test::SimpleNode
";
	#[cfg(feature = "tynm")]
	let pattern = r"resolves BuilderSimpleNode -> BuilderLeaf
built #0.0  BuilderLeaf => Leaf
built #0.1  BuilderSimpleNode => SimpleNode
resolves BuilderSimpleNode -> BuilderLeaf
built #0.2  BuilderSimpleNode => SimpleNode
";

	// Textual output storage
	let mut data = Vec::new();
	
	let mut cache = ArtifactCache::new_with_doctor(
		diagnostics::TextualDoc::new(
			diagnostics::TextualDocOptions {
				show_builder_values: false,
				show_artifact_values: false,
				show_addresses: false,
				tynm_m_n: Some((0,0)),
			},
			data
		)
	);
	
	
	// Test data
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let node1 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf1.clone()));
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&node2), cache.get(&node2));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&node1), cache.get(&node2));
	
	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get::<BuilderSimpleNode>(&node2).leaf, cache.get(&node1).leaf);
	
	// Get the vector back, dissolves cache & doctor
	data = cache.into_doctor().into_inner();
	
	let string = String::from_utf8(data).unwrap();
	// Print the resulting string, very usable in case it does not match
	println!("{}", string);
	
	assert_eq!(pattern, &string);
}

#[test]
fn test_complex_clear() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let nodef1 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	
	let noden1 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));
	
	let artifact_leaf = cache.get(&leaf1);
	let artifact_node = cache.get(&noden1);
	let artifact_root = cache.get(&noden3);
	
	cache.clear();
	
	let artifact_root_2 = cache.get(&noden3);
	let artifact_node_2 = cache.get(&noden1);
	let artifact_leaf_2 = cache.get(&leaf1);
	
	assert_ne!(artifact_leaf, artifact_leaf_2);
	assert_ne!(artifact_node, artifact_node_2);
	assert_ne!(artifact_root, artifact_root_2);
	
}

#[test]
fn test_invalidate() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let artifact1 = cache.get(&leaf1);
	
	cache.invalidate(leaf1.clone());
	
	let artifact2 = cache.get(&leaf1);
	
	// Ensure artifacts differ after clear
	assert_ne!(artifact1, artifact2);
	
}

#[test]
fn test_into() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	let lart = cache.get(&leaf1);
	
	let node1 = ArtifactPromiseRc::new(BuilderSimpleNode::new(leaf1));
	
	assert_eq!(cache.get(&node1).leaf.as_ref(), lart.as_ref());
}

#[test]
fn test_complex_invalidate() {
	let mut cache = ArtifactCacheRc::new();
	
	let leaf1 = ArtifactPromiseRc::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromiseRc::new(BuilderLeaf::new());
	
	let nodef1 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = ArtifactPromiseRc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	
	let noden1 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = ArtifactPromiseRc::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));
	
	let artifact_leaf = cache.get(&leaf1);
	let artifact_node = cache.get(&noden1);
	let artifact_root = cache.get(&noden3);
	
	// Only invalidate one intermediate node
	cache.invalidate(noden1.clone());
	
	let artifact_leaf_2 = cache.get(&leaf1);
	let artifact_node_2 = cache.get(&noden1);
	let artifact_root_2 = cache.get(&noden3);
	
	assert_eq!(artifact_leaf, artifact_leaf_2);
	assert_ne!(artifact_node, artifact_node_2);
	assert_ne!(artifact_root, artifact_root_2);
	
}




