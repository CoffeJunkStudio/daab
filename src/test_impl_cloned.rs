


use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU32;
use pretty_assertions::{assert_eq, assert_ne};

use super::*;

use crate::Unpacking;

#[cfg(feature = "diagnostics")]
use crate::diagnostics;

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


#[derive(Debug, Clone, PartialEq, Eq)]
struct SimpleNode {
	id: u32,
	leaf: Leaf,
}

#[derive(Debug, Clone)]
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
		let leaf = cache.resolve_cloned(&self.leaf).unpack();

		SimpleNode{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
			leaf
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum LeafOrNodes {
	Leaf(Leaf),
	Nodes {
		left: ComplexNode,
		right: ComplexNode
	},
}

#[derive(Debug, Clone)]
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
				LeafOrNodes::Leaf(cache.resolve_cloned(l).unpack())
			},
			Self::Nodes{left, right} => {
				LeafOrNodes::Nodes{
					left: cache.resolve_cloned(left).unpack(),
					right: cache.resolve_cloned(right).unpack(),
				}
			},
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ComplexNode {
	id: u32,
	inner: Box<LeafOrNodes>,
}

impl ComplexNode {
	pub(crate) fn leaf(&self) -> Option<&Leaf> {
		if let LeafOrNodes::Leaf(ref l) = *self.inner {
			Some(l)
		} else {
			None
		}
	}

	pub(crate) fn left(&self) -> Option<&ComplexNode> {
		if let LeafOrNodes::Nodes{ref left, ..} = *self.inner {
			Some(left)
		} else {
			None
		}
	}

	pub(crate) fn right(&self) -> Option<&ComplexNode> {
		if let LeafOrNodes::Nodes{ref right, ..} = *self.inner {
			Some(right)
		} else {
			None
		}
	}
}

#[derive(Debug, Clone)]
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
			inner: Box::new(self.inner.build(cache)),
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
	assert_eq!(cache.get_cloned(&leaf1), cache.get_cloned(&leaf1));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&leaf1), cache.get_cloned(&leaf2));
}

#[test]
fn test_leaf() {
	let mut cache = Cache::new();

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
fn test_node() {
	let mut cache = Cache::new();

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
	assert_eq!(cache.get_cloned(&node2).unpack().leaf, cache.get_cloned(&node3).unpack().leaf);

}

#[test]
fn test_complex() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = Blueprint::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = Blueprint::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_cloned(&noden3), cache.get_cloned(&noden3));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&noden1), cache.get_cloned(&noden2));

	let artifact_leaf = cache.get_cloned(&leaf1).unpack();
	let artifact_node = cache.get_cloned(&noden1).unpack();
	let artifact_root = cache.get_cloned(&noden3).unpack();

	assert_eq!(artifact_root.left(), artifact_root.right());

	assert_eq!(artifact_root.left().unwrap().right(), Some(&artifact_node));
	assert_eq!(artifact_node.left().unwrap().leaf(), Some(&artifact_leaf));

}

#[test]
fn test_clear() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());

	let artifact1 = cache.get_cloned(&leaf1);

	cache.clear_all();

	let artifact2 = cache.get_cloned(&leaf1);

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


	let mut cache = Cache::new_with_doctor(visgraph_doc(data));

	// Test data
	let leaf1 = Blueprint::new(BuilderLeaf::new());

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_cloned(&node2), cache.get_cloned(&node2));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&node1), cache.get_cloned(&node2));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get_cloned::<_, BuilderSimpleNode>(&node2).unpack().leaf, cache.get_cloned(&node1).unpack().leaf);

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

	let mut cache = Cache::new_with_doctor(
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
	let leaf1 = Blueprint::new(BuilderLeaf::new());

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_cloned(&node2), cache.get_cloned(&node2));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&node1), cache.get_cloned(&node2));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get_cloned::<_, BuilderSimpleNode>(&node2).unpack().leaf, cache.get_cloned(&node1).unpack().leaf);

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

	let mut cache = Cache::new_with_doctor(
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
	let leaf1 = Blueprint::new(BuilderLeaf::new());

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));
	let node2 = Blueprint::new(BuilderSimpleNode::new(leaf1.clone()));

	// Ensure same builder results in same artifact
	assert_eq!(cache.get_cloned(&node2), cache.get_cloned(&node2));

	// Ensure different builder result in different artifacts
	assert_ne!(cache.get_cloned(&node1), cache.get_cloned(&node2));

	// Enusre that different artifacts may link the same dependent artifact
	assert_eq!(cache.get_cloned::<_, BuilderSimpleNode>(&node2).unpack().leaf, cache.get_cloned(&node1).unpack().leaf);

	// Get the vector back, dissolves cache & doctor
	data = cache.into_doctor().into_inner();

	let string = String::from_utf8(data).unwrap();
	// Print the resulting string, very usable in case it does not match
	println!("{}", string);

	assert!(regex.is_match(&string));
}

#[test]
fn test_complex_clear() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = Blueprint::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = Blueprint::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));

	let artifact_leaf = cache.get_cloned(&leaf1);
	let artifact_node = cache.get_cloned(&noden1);
	let artifact_root = cache.get_cloned(&noden3);

	cache.clear_all();

	let artifact_root_2 = cache.get_cloned(&noden3);
	let artifact_node_2 = cache.get_cloned(&noden1);
	let artifact_leaf_2 = cache.get_cloned(&leaf1);

	assert_ne!(artifact_leaf, artifact_leaf_2);
	assert_ne!(artifact_node, artifact_node_2);
	assert_ne!(artifact_root, artifact_root_2);

}

#[test]
fn test_invalidate() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());

	let artifact1 = cache.get_cloned(&leaf1);

	cache.invalidate(&leaf1);

	let artifact2 = cache.get_cloned(&leaf1);

	// Ensure artifacts differ after clear
	assert_ne!(artifact1, artifact2);

}

#[test]
fn test_into() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let lart = cache.get_cloned(&leaf1).unpack();

	let node1 = Blueprint::new(BuilderSimpleNode::new(leaf1));

	assert_eq!(cache.get_cloned(&node1).unpack().leaf, lart);
}

#[test]
fn test_complex_invalidate() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));
	let nodef3 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
	let noden2 = Blueprint::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
	let noden3 = Blueprint::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));

	let artifact_leaf = cache.get_cloned(&leaf1);
	let artifact_node = cache.get_cloned(&noden1);
	let artifact_root = cache.get_cloned(&noden3);

	// Only invalidate one intermediate node
	cache.invalidate(&noden1);

	let artifact_leaf_2 = cache.get_cloned(&leaf1);
	let artifact_node_2 = cache.get_cloned(&noden1);
	let artifact_root_2 = cache.get_cloned(&noden3);

	assert_eq!(artifact_leaf, artifact_leaf_2);
	assert_ne!(artifact_node, artifact_node_2);
	assert_ne!(artifact_root, artifact_root_2);

}

/*
Does not work with Arc!
#[test]
fn test_dyn_builder_stable() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));

	let artifact_node = cache.get_cloned(&noden1);


	let noden1_unsized: DynamicBlueprint<ComplexNode> = noden1.into();

	assert_eq!(artifact_node, cache.get_cloned(&noden1_unsized));

	// Try it again
	assert_eq!(artifact_node, cache.get_cloned(&noden1_unsized));

}
*/

/*
Does not work with Arc!
#[test]
fn test_dyn_builder_stable2() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1_builder =
		BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone());
	//let noden1 = Blueprint::new_binned(noden1_rc.clone());

	//let artifact_node = cache.get_cloned(&noden1);


	let noden1_unsized: DynamicBlueprint<ComplexNode> = DynamicBlueprint::new_unsized(noden1_builder);

	assert_eq!(cache.get_cloned(&noden1_unsized), cache.get_cloned(&noden1_unsized));

	// Try it again
	assert_eq!(cache.get_cloned(&noden1_unsized), cache.get_cloned(&noden1_unsized));

}
*/

#[cfg(feature = "unsized")]
#[test]
fn test_dyn_builder() {
	let mut cache = Cache::new();

	let leaf1 = Blueprint::new(BuilderLeaf::new());
	let leaf2 = Blueprint::new(BuilderLeaf::new());

	let nodef1 = Blueprint::new(BuilderComplexNode::new_leaf(leaf1.clone()));
	let nodef2 = Blueprint::new(BuilderComplexNode::new_leaf(leaf2.clone()));

	let noden1 = Blueprint::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));

	let artifact_node = cache.get_cloned(&noden1);


	let noden1_unsized: DynamicBlueprint<ComplexNode> = noden1.clone().into_unsized();

	assert_eq!(artifact_node, cache.get_cloned(&noden1_unsized));

	// Try it again
	assert_eq!(artifact_node, cache.get_cloned(&noden1_unsized));

}

#[cfg(feature = "unsized")]
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

	let artifact_node = cache.get_cloned(&noden1);
	let artifact_root = cache.get_cloned(&noden3);

	let mut unsized_vec = Vec::new();

	let noden1_unsized: DynamicBlueprint<ComplexNode> = noden1.clone().into_unsized();
	assert_eq!(cache.get_cloned(&noden1), cache.get_cloned(&noden1_unsized));
	unsized_vec.push(noden1_unsized);

	let noden2_unsized: DynamicBlueprint<ComplexNode> = noden2.clone().into_unsized();
	assert_eq!(cache.get_cloned(&noden2), cache.get_cloned(&noden2_unsized));
	unsized_vec.push(noden2_unsized);

	let noden3_unsized: DynamicBlueprint<ComplexNode> = noden3.clone().into_unsized();
	assert_eq!(cache.get_cloned(&noden3), cache.get_cloned(&noden3_unsized));
	unsized_vec.push(noden3_unsized);


	let artifact_vec: Vec<_> = unsized_vec.iter().map( |ap|
		cache.get_cloned(ap)
	).collect();

	unsized_vec.iter().zip(artifact_vec.into_iter()).for_each( |(ap,art)| {
		assert_eq!(
			cache.get_cloned(ap),
			art
		);
	});

}



