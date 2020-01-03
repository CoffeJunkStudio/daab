

use std::rc::Rc;
use std::collections::HashMap;
use std::any::Any;


/// Represents a builder for an artifact.
///
/// Each builder is supposed to contain all direct depenencies possibly other
/// builders.
/// In the `build()` function, the builder can access the cache inorder to
/// resolve depending builders to their artifact.
///
pub trait Builder {
    type Artifact;
    
    fn build(&self, cache: &mut ArtifactCache) -> Rc<Self::Artifact>;
}



/// Central structure to prevent dependency duplication on building.
///
pub struct ArtifactCache {
	// Maps Builder-ptr to their Output value
	cache: HashMap<*const usize, Rc<dyn Any>>,
	// Stores all Builder Rcs reference in above map, to ensure that the above pointer remain valid
	builder_blockage: Vec<Rc<dyn Any>>,
}

impl Default for ArtifactCache {
	fn default() -> Self {
		ArtifactCache::new()
	}
}

impl ArtifactCache {
	
	/// Creates new empty cache
	///
	pub fn new() -> Self {
		Self {
			cache: HashMap::new(),
			builder_blockage: Vec::new(),
		}
	}
	
	/// Returns the raw address of the inner allocation of the given builder Rc.
	///
	fn builder_ptr<B: Builder>(builder: &Rc<B>) -> *const usize {
		builder as &B as *const B as *const usize
	}
	
	/// Get the stored artifact if it exists.
	///
	fn lookup<B: Builder>(&self, builder: &Rc<B>) -> Option<Rc<B::Artifact>>
			where <B as Builder>::Artifact: 'static {
		
		let ptr = Self::builder_ptr(builder);
		
		// Get the artifact from the hash map ensuring integrity
		self.cache.get(&ptr).map(
			|rc| {
				// Ensure value type
				rc.clone().downcast()
					.expect("Cached Builder Artifact is of invalid type")
			}
		)
	}
	
	/// Store given artifact for given builder.
	///
	fn insert<B: Builder + 'static>(&mut self, builder: Rc<B>, artifact: Rc<B::Artifact>) {
		
		// Insert artifact
		self.cache.insert(
			Self::builder_ptr(&builder),
			artifact
		);
		
		// Bloc builder for deallocation
		self.builder_blockage.push(builder);
		
	}
	
	/// Gets the artifact of the given builder.
	///
	/// This method looksup whether the artifact for the given builder is still
	/// present in the cache, or it will use the builder to build and store the
	/// artifact.
	///
	/// Notice the given builder will be stored keept to prevent it from
	/// deallocating. `clear()` must be called inorder to free those Rcs.
	///
	pub fn get<B: Builder + 'static>(&mut self, builder: &Rc<B>) -> Rc<B::Artifact>
			where <B as Builder>::Artifact: 'static {
		
		if let Some(rc) = self.lookup(builder) {
			rc
			
		} else {
			let rc = builder.build(self);
			
			self.insert(builder.clone(), rc.clone());
			
			rc
		}
	}
	
	/// Clears the entire cache including all hold builder Rcs.
	///
	pub fn clear(&mut self) {
		self.cache.clear();
		self.builder_blockage.clear();
	}
}





// -----------

#[cfg(test)]
mod tests {
	use super::*;
	
	use std::rc::Rc;
	use std::sync::atomic::Ordering;
	use std::sync::atomic::AtomicU32;
	
	
	// Dummy counter to differentiate the leaf instances
	static counter: AtomicU32 = AtomicU32::new(0);

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

	impl Builder for BuilderLeaf {
		type Artifact = Leaf;
		
		fn build(&self, cache: &mut ArtifactCache) -> Rc<Self::Artifact> {
		    Rc::new(Leaf{
				id: counter.fetch_add(1, Ordering::SeqCst),
			})
		}
	}


	#[derive(Debug, PartialEq, Eq)]
	struct SimpleNode {
		id: u32,
		leaf: Rc<Leaf>,
	}

	#[derive(Debug)]
	struct BuilderSimpleNode {
		leaf: Rc<BuilderLeaf>,
	}

	impl BuilderSimpleNode {
		pub fn new(leaf: Rc<BuilderLeaf>) -> Self {
		    Self {
		        leaf,
		    }
		}
	}

	impl Builder for BuilderSimpleNode {
		type Artifact = SimpleNode;
		
		fn build(&self, cache: &mut ArtifactCache) -> Rc<Self::Artifact> {
			let leaf = cache.get(&self.leaf);
		    
		    Rc::new(SimpleNode{
		    	id: counter.fetch_add(1, Ordering::SeqCst),
		    	leaf
		    })
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
		Leaf(Rc<BuilderLeaf>),
		Nodes {
			left: Rc<BuilderComplexNode>,
			right: Rc<BuilderComplexNode>
		},
	}
	
	impl BuilderLeafOrNodes {
		fn build(&self, cache: &mut ArtifactCache) -> LeafOrNodes {
			match self {
				Self::Leaf(l) => {
					LeafOrNodes::Leaf(cache.get(l))
				},
				Self::Nodes{left, right} => {
					LeafOrNodes::Nodes{
						left: cache.get(left),
						right: cache.get(right),
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

	#[derive(Debug)]
	struct BuilderComplexNode {
		inner: BuilderLeafOrNodes,
	}

	impl BuilderComplexNode {
		pub fn new_leaf(leaf: Rc<BuilderLeaf>) -> Self {
		    Self {
		        inner: BuilderLeafOrNodes::Leaf(leaf),
		    }
		}
		
		pub fn new_nodes(left: Rc<BuilderComplexNode>, right: Rc<BuilderComplexNode>) -> Self {
		    Self {
		        inner: BuilderLeafOrNodes::Nodes{left, right},
		    }
		}
	}

	impl Builder for BuilderComplexNode {
		type Artifact = ComplexNode;
		
		fn build(&self, cache: &mut ArtifactCache) -> Rc<Self::Artifact> {
		    Rc::new(ComplexNode{
		    	id: counter.fetch_add(1, Ordering::SeqCst),
		    	inner: self.inner.build(cache),
		    })
		}
	}
    
    #[test]
	fn test_leaf() {
		let mut cache = ArtifactCache::new();
		
		let leaf1 = Rc::new(BuilderLeaf::new());
		let leaf2 = Rc::new(BuilderLeaf::new());
		
		println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
		
		// Ensure same builder results in same artifact
		assert_eq!(cache.get(&leaf1), cache.get(&leaf1));
		
		// Ensure different builder result in  different artifacts
		assert_ne!(cache.get(&leaf1), cache.get(&leaf2));
	}
    
    #[test]
	fn test_node() {
		let mut cache = ArtifactCache::new();
		
		let leaf1 = Rc::new(BuilderLeaf::new());
		let leaf2 = Rc::new(BuilderLeaf::new());
		
		let node1 = Rc::new(BuilderSimpleNode::new(leaf1.clone()));
		let node2 = Rc::new(BuilderSimpleNode::new(leaf2.clone()));
		let node3 = Rc::new(BuilderSimpleNode::new(leaf2.clone()));
		
		// Ensure same builder results in same artifact
		assert_eq!(cache.get(&node1), cache.get(&node1));
		
		// Ensure different builder result in  different artifacts
		assert_ne!(cache.get(&node2), cache.get(&node3));
		
		// Enusre that different artifacts may link the same dependent artifact
		assert_eq!(cache.get(&node2).leaf, cache.get(&node3).leaf);
		
	}
    
    #[test]
	fn test_complex() {
		let mut cache = ArtifactCache::new();
		
		let leaf1 = Rc::new(BuilderLeaf::new());
		let leaf2 = Rc::new(BuilderLeaf::new());
		
		let nodef1 = Rc::new(BuilderComplexNode::new_leaf(leaf1.clone()));
		let nodef2 = Rc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
		let nodef3 = Rc::new(BuilderComplexNode::new_leaf(leaf2.clone()));
		
		let noden1 = Rc::new(BuilderComplexNode::new_nodes(nodef1.clone(), nodef2.clone()));
		let noden2 = Rc::new(BuilderComplexNode::new_nodes(nodef3.clone(), noden1.clone()));
		let noden3 = Rc::new(BuilderComplexNode::new_nodes(noden2.clone(), noden2.clone()));
		
		// Ensure same builder results in same artifact
		assert_eq!(cache.get(&noden3), cache.get(&noden3));
		
		// Ensure different builder result in  different artifacts
		assert_ne!(cache.get(&noden1), cache.get(&noden2));
		
		
	}
}





