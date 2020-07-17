


use std::borrow::Borrow;
use std::fmt;
use std::fmt::Debug;
use std::hash::Hash;
use std::hash::Hasher;

use crate::ArtifactPromiseTrait;

use crate::BuilderId;


/// Auxiliary struct fro the `ArtifactCache` containing an untyped (aka
/// `dyn Any`) ArtifactPromise.
///
#[derive(Clone, Debug)]
pub struct BuilderEntry<BCan> {
	builder: BCan,
	id: BuilderId,
}

impl<BCan> BuilderEntry<BCan> {
	pub fn new<AP, B: ?Sized + 'static>(ap: &AP) -> Self
			where AP: ArtifactPromiseTrait<B, BCan> {

		let id = ap.id();

		BuilderEntry {
			builder: ap.canned().can,
			id,
		}
	}

	pub fn id(&self) -> BuilderId {
		self.id
	}
}

impl<BCan> Hash for BuilderEntry<BCan> {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.id.hash(state);
	}
}

impl<BCan> PartialEq for BuilderEntry<BCan> {
	fn eq(&self, other: &Self) -> bool {
		self.id.eq(&other.id)
	}
}

impl<BCan> Eq for BuilderEntry<BCan> {
}

impl<BCan> Borrow<BuilderId> for BuilderEntry<BCan> {
	fn borrow(&self) -> &BuilderId {
		&self.id
	}
}

impl<BCan> fmt::Pointer for BuilderEntry<BCan> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		writeln!(f, "{:p}", self.id.0)
	}
}


