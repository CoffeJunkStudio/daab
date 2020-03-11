


use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU32;
use pretty_assertions::{assert_eq, assert_ne};

use super::*;

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
	pub fn new() -> Self {
		Self {
			// empty
		}
	}
}

impl SimpleBuilder for BuilderLeaf {
	type Artifact = Leaf;
	
	fn build(&self, _cache: &mut ArtifactResolver) -> Self::Artifact {
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
	leaf: ArtifactPromise<BuilderLeaf>,
}

impl BuilderSimpleNode {
	pub fn new(leaf: ArtifactPromise<BuilderLeaf>) -> Self {
		Self {
			leaf,
		}
	}
}

impl SimpleBuilder for BuilderSimpleNode {
	type Artifact = SimpleNode;
	
	fn build(&self, cache: &mut ArtifactResolver) -> Self::Artifact {
		let leaf = cache.resolve(&self.leaf);
		
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
	Leaf(ArtifactPromise<BuilderLeaf>),
	Nodes {
		left: ArtifactPromise<BuilderComplexNode>,
		right: ArtifactPromise<BuilderComplexNode>
	},
}

// Fixes in the Arc case:
// error[E0275]: overflow evaluating the requirement
// `std::sync::Arc<(dyn std::any::Any + std::marker::Send + std::marker::Sync + 'static)>: canning::Can<test_arc::BuilderComplexNode>`
unsafe impl Send for BuilderLeafOrNodes {}
unsafe impl Sync for BuilderLeafOrNodes {}

impl BuilderLeafOrNodes {
	fn build(&self, cache: &mut ArtifactResolver) -> LeafOrNodes {
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
	pub fn leaf(&self) -> Option<&BinType<Leaf>> {
		if let LeafOrNodes::Leaf(ref l) = self.inner {
			Some(l)
		} else {
			None
		}
	}
	
	pub fn left(&self) -> Option<&BinType<ComplexNode>> {
		if let LeafOrNodes::Nodes{ref left, ..} = self.inner {
			Some(left)
		} else {
			None
		}
	}
	
	pub fn right(&self) -> Option<&BinType<ComplexNode>> {
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
	pub fn new_leaf(leaf: ArtifactPromise<BuilderLeaf>) -> Self {
		Self {
			inner: BuilderLeafOrNodes::Leaf(leaf),
		}
	}
	
	pub fn new_nodes(left: ArtifactPromise<BuilderComplexNode>, right: ArtifactPromise<BuilderComplexNode>) -> Self {
		Self {
			inner: BuilderLeafOrNodes::Nodes{left, right},
		}
	}
}

impl SimpleBuilder for BuilderComplexNode {
	type Artifact = ComplexNode;
	
	fn build(&self, cache: &mut ArtifactResolver) -> Self::Artifact {
		ComplexNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			inner: self.inner.build(cache),
		}
	}
}

#[test]
fn test_leaf_broken() {
	let mut cache = ArtifactCache::new();
	
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
	let mut cache = ArtifactCache::new();
	
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
fn test_node() {
	let mut cache = ArtifactCache::new();
	
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
fn test_complex() {
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());
	
	let nodef1 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	
	let noden1 = ArtifactPromise::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = ArtifactPromise::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = ArtifactPromise::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));
	
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
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	
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

#[allow(dead_code)]
const VIS_DOC_PATTERN: &str = r##"strict digraph \{ graph \[labeljust = l\];
  "0x[0-9a-f]+" \[label = "daab::.+::BuilderSimpleNode"\]
  "0x[0-9a-f]+" \[label = "daab::.+::BuilderLeaf"\]
  "0x[0-9a-f]+" -> "0x[0-9a-f]+"
  "0x[0-9a-f]+" \[label = "daab::.+::BuilderLeaf"\]
  "0\.0-0x[0-9a-f]+" \[label = "#0\.0 daab::.+::Leaf :
Leaf \{
    id: [0-9]+,
\}", shape = box\]
  "0x[0-9a-f]+" -> "0.0-0x[0-9a-f]+" \[arrowhead = "none"\]
  "0x[0-9a-f]+" \[label = "daab::.+::BuilderSimpleNode"\]
  "0\.1-0x[0-9a-f]+" \[label = "#0\.1 daab::.+::SimpleNode :
SimpleNode \{
    id: [0-9]+,
    leaf: Leaf \{
        id: [0-9]+,
    \},
\}", shape = box\]
  "0x[0-9a-f]+" -> "0.1-0x[0-9a-f]+" \[arrowhead = "none"\]
  "0x[0-9a-f]+" \[label = "daab::.+::BuilderSimpleNode"\]
  "0x[0-9a-f]+" \[label = "daab::.+::BuilderLeaf"\]
  "0x[0-9a-f]+" -> "0x[0-9a-f]+"
  "0x[0-9a-f]+" \[label = "daab::.+::BuilderSimpleNode"\]
  "0\.2-0x[0-9a-f]+" \[label = "#0\.2 daab::.+::SimpleNode :
SimpleNode \{
    id: [0-9]+,
    leaf: Leaf \{
        id: [0-9]+,
    \},
\}", shape = box\]
  "0x[0-9a-f]+" -> "0.2-0x[0-9a-f]+" \[arrowhead = "none"\]
\}"##;

#[test]
#[cfg(feature = "diagnostics")]
fn test_vis_doc() {
	
	// Expected value as Regular Expression due to variable addresses and counters
	let regex = regex::Regex::new(VIS_DOC_PATTERN).unwrap();


	// Visgraph output storage
	let mut data = Vec::new();
	
	
	let mut cache = ArtifactCache::new_with_doctor(visgraph_doc(data));
	
	// Test data
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	
	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	
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

#[allow(dead_code)]
const TEXT_DOC_PATTERN_STD: &str = r"resolves daab::.+::BuilderSimpleNode -> daab::.+::BuilderLeaf
built #0.0  daab::.+::BuilderLeaf => daab::.+::Leaf
built #0.1  daab::.+::BuilderSimpleNode => daab::.+::SimpleNode
resolves daab::.+::BuilderSimpleNode -> daab::.+::BuilderLeaf
built #0.2  daab::.+::BuilderSimpleNode => daab::.+::SimpleNode
";

#[allow(dead_code)]
const TEXT_DOC_PATTERN_TYNM: &str = r"resolves BuilderSimpleNode -> BuilderLeaf
built #0.0  BuilderLeaf => Leaf
built #0.1  BuilderSimpleNode => SimpleNode
resolves BuilderSimpleNode -> BuilderLeaf
built #0.2  BuilderSimpleNode => SimpleNode
";


#[test]
#[cfg(feature = "diagnostics")]
fn test_text_doc() {
	
	// Expected value as Regular Expression due to variable addresses and counters
	#[cfg(not(feature = "tynm"))]
	let regex = regex::Regex::new(TEXT_DOC_PATTERN_STD).unwrap();
	#[cfg(feature = "tynm")]
	let regex = regex::Regex::new(TEXT_DOC_PATTERN_TYNM).unwrap();

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
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	
	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	
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
	
	assert!(regex.is_match(&string));
}

#[test]
#[cfg(feature = "diagnostics")]
fn test_text_doc_long() {
	
	// Expected value as Regular Expression due to variable addresses and counters
	let regex = regex::Regex::new(TEXT_DOC_PATTERN_STD).unwrap();

	// Textual output storage
	let mut data = Vec::new();
	
	let mut cache = ArtifactCache::new_with_doctor(
		diagnostics::TextualDoc::new(
			diagnostics::TextualDocOptions {
				show_builder_values: false,
				show_artifact_values: false,
				show_addresses: false,
				// TODO use when newer version in avaiable
				//tynm_m_n: Some((std::usize::MAX,std::usize::MAX)),
				tynm_m_n: Some((100,100)),
			},
			data
		)
	);
	
	
	// Test data
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	
	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1.clone()));
	
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
	
	assert!(regex.is_match(&string));
}

#[test]
fn test_complex_clear() {
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());
	
	let nodef1 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	
	let noden1 = ArtifactPromise::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = ArtifactPromise::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = ArtifactPromise::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));
	
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
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	
	let artifact1 = cache.get(&leaf1);
	
	cache.invalidate(leaf1.clone());
	
	let artifact2 = cache.get(&leaf1);
	
	// Ensure artifacts differ after clear
	assert_ne!(artifact1, artifact2);
	
}

#[test]
fn test_into() {
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let lart = cache.get(&leaf1);
	
	let node1 = ArtifactPromise::new(BuilderSimpleNode::new(leaf1));
	
	assert_eq!(cache.get(&node1).leaf.as_ref(), lart.as_ref());
}

#[test]
fn test_complex_invalidate() {
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf::new());
	let leaf2 = ArtifactPromise::new(BuilderLeaf::new());
	
	let nodef1 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = ArtifactPromise::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	
	let noden1 = ArtifactPromise::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = ArtifactPromise::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = ArtifactPromise::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));
	
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




