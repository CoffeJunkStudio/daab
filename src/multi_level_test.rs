use super::*;

use std::sync::atomic::Ordering;
use std::sync::atomic::AtomicU32;



// Dummy counter to differentiate instances
static COUNTER: AtomicU32 = AtomicU32::new(0);

// Comparable leaf
#[derive(Debug, PartialEq, Eq)]
struct Leaf {
	id: u32,
}

// Simple builder for Leaf
#[derive(Debug)]
struct BuilderLeaf {
	// empty
}

impl rc::SimpleBuilder for BuilderLeaf {
	type Artifact = Leaf;
	
	fn build(&self, _cache: &mut rc::ArtifactResolver) -> Self::Artifact {
		Leaf{
			id: COUNTER.fetch_add(1, Ordering::SeqCst),
		}
	}
}


// Builds the builder of Leaf, first level indirection
#[derive(Debug)]
struct BuilderBuilder {
	
}

impl rc::SuperBuilder for BuilderBuilder {
	type Artifact = BuilderLeaf;
	type DynState = ();

	fn build(&self, _cache: &mut rc::SuperArtifactResolver) -> Self::Artifact {
		BuilderLeaf{}
	}
}


// Second level indirection builder of Leaf
#[derive(Debug)]
struct SuperBuilder {
	
}

impl rc::SuperBuilder for SuperBuilder {
	type Artifact = BuilderBuilder;
	type DynState = ();

	fn build(&self, _cache: &mut rc::SuperArtifactResolver) -> Self::Artifact {
		BuilderBuilder{}
	}
}



// Base line test
#[test]
fn test_builder_leaf() {
	let mut cache = rc::ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf{});
	let leaf2 = ArtifactPromise::new(BuilderLeaf{});
	
	//println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&leaf1), cache.get(&leaf2));
}

// Test for first level indirection
#[test]
fn test_level_1() {
	let mut cache_ap = ArtifactCache::new();
	
	let bb1 = ArtifactPromise::new(BuilderBuilder{});
	
	
	let mut cache = ArtifactCache::new();
	
	let leaf1 = cache_ap.get(&bb1);
	let leaf2 = ArtifactPromise::new(BuilderLeaf{});
	
	//println!("BuilderLeaf: {:?}; {:?}", leaf1, leaf2);
	
	// Ensure same builder results in same artifact
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));
	
	// Ensure different builder result in different artifacts
	assert_ne!(cache.get(&leaf1), cache.get(&leaf2));
	
	assert_eq!(cache.get(&leaf1), cache.get(&cache_ap.get(&bb1)));
	
	
	// Just clear builder builder
	cache_ap.clear();
	
	assert_ne!(cache.get(&leaf1), cache.get(&cache_ap.get(&bb1)))
}


// Test for second level indirection (same cache)
#[test]
fn test_level_2() {
	let mut cache_ap = ArtifactCache::new();
	
	let sb1 = ArtifactPromise::new(SuperBuilder{});
	
	let bb1 = ArtifactPromise::new(BuilderBuilder{});
	
	
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf{});
	
	// Flat test
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));
	
	
	// Level 1 test
	assert_ne!(cache.get(&cache_ap.get(&bb1)), cache.get(&leaf1));
	
	assert_eq!(cache.get(&cache_ap.get(&bb1)), cache.get(&cache_ap.get(&bb1)));
	
	// Level 2 test (need temporaries for recursive lookup)
	let l1 = cache_ap.get(&sb1);
	assert_ne!(cache.get(&cache_ap.get(&l1)), cache.get(&leaf1));
	
	let l1 = cache_ap.get(&sb1);
	assert_ne!(cache.get(&cache_ap.get(&l1)), cache.get(&cache_ap.get(&bb1)));
	
	let l1 = cache_ap.get(&sb1);
	let l2 = cache_ap.get(&sb1);
	assert_eq!(cache.get(&cache_ap.get(&l1)), cache.get(&cache_ap.get(&l2)));
	
}


// Test for second level indirection (different caches)
#[test]
fn test_level_2_diff_caches() {
	let mut cache_ap1 = ArtifactCache::new();
	let mut cache_ap2 = ArtifactCache::new();
	
	let sb1 = ArtifactPromise::new(SuperBuilder{});
	
	let bb1 = ArtifactPromise::new(BuilderBuilder{});
	
	
	let mut cache = ArtifactCache::new();
	
	let leaf1 = ArtifactPromise::new(BuilderLeaf{});
	
	// Flat test
	assert_eq!(cache.get(&leaf1), cache.get(&leaf1));
	
	
	// Level 1 test
	assert_ne!(cache.get(&cache_ap1.get(&bb1)), cache.get(&leaf1));
	
	assert_eq!(cache.get(&cache_ap1.get(&bb1)), cache.get(&cache_ap1.get(&bb1)));
	
	// Level 2 test
	assert_ne!(cache.get(&cache_ap1.get(&cache_ap2.get(&sb1))), cache.get(&leaf1));
	
	assert_ne!(cache.get(&cache_ap1.get(&cache_ap2.get(&sb1))), cache.get(&cache_ap1.get(&bb1)));
	
	assert_eq!(cache.get(&cache_ap1.get(&cache_ap2.get(&sb1))), cache.get(&cache_ap1.get(&cache_ap2.get(&sb1))));
	
}




