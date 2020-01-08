
//!
//! # Extensive debugging analysis module.
//!
//! **Notice: This module is only available if the `diagnostics` feature has been activated**.
//!
//! This module contains the types used in debugging the [`ArtifactCache`].
//! The most important one is [`Doctor`] which conducts a diagnosis on a
//! `ArtifactCache` if constructed via [`ArtifactCache::new_with_doctor()`].
//!
//! `Doctor` has methods for various events happening in the `ArtifactCache` getting the relevant builder or artifact as argument.
//! See the respective method of the `Doctor`.
//!
//! Additionally, to the generic `Doctor` trait, there are several pre-implemented
//! Doctors such as: `VisgraphDoc`.
//!
//! [`ArtifactCache`]: ../struct.ArtifactCache.html
//! [`Doctor`]: trait.Doctor.html
//! [`ArtifactCache::new_with_doctor()`]: ../struct.ArtifactCache.html#method.new_with_doctor

// TODO:
// - Printout Doctor
// - visgraph options Display
// - doctor default impl
// - doctor additional functions
// - doctor special artifact/builder wrapper

use std::io::Write;

use super::BuilderEntry;
use super::ArtifactEntry;

/// `ArtifactCache` Debugger.
///
/// **Notice: This trait is only available if the `diagnostics` feature has been activated**.
///
/// The Doctor conducts diagnoses on the `ArtifactCache`, if it is passed
/// with [`ArtifactCache::new_with_doctor()`]. The `ArtifactCache` will
/// call the methods of this trait whenever the respective event happens.
/// It will be supplied with relevant object(s), such as `Builder`s and artifacts.
/// For details on each event see the respective method.
///
/// Each method as a default implementation to ease implementing specialized `Doctor`s which don't need all the events. Each default implementation just dose nothing, i.e. are no-ops.
///
/// [`ArtifactCache::new_with_doctor()`]: ../struct.ArtifactCache.html#method.new_with_doctor
///
pub trait Doctor {
	/// One `Builder` resolves another `Builder`.
	///
	/// This methods means that `builder` appearently depends on `used`.
	///
	fn resolve(&mut self, _builder: &BuilderEntry, _used: &BuilderEntry) {
		// NOOP
	}
	
	/// One `Builder` builds its artifact.
	///
	/// This method is called each time `builder` is invoked to build
	/// its `artifact`. Notice, this function is only called when a fresh
	/// artifact is actually constructed, i.e. first time it is resolved
	/// or when it is resolved after a reset or invalidation.
	///
	fn build(&mut self, _builder: &BuilderEntry, _artifact: &ArtifactEntry) {
		// NOOP
	}
}

/// Output options for `VisgrapDoc`.
///
/// **Notice: This struc is only available if the `diagnostics` feature has been activated**.
///
/// This struct contains outputting options for the `VisgraphDoc`.
///
/// It has a `Default` impl with the following value:
/// ```
/// # use daab::diagnostics::VisgraphDocOptions;
/// // Value of default()
/// let opts = VisgraphDocOptions {
///	    show_builder_values: false,
///	    show_artifact_values: true,
/// };
/// assert_eq!(opts, VisgraphDocOptions::default());
/// ```
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VisgraphDocOptions {
	/// Configures whether builders should be only visualized by their type (`false`) or
	/// by their value (`true`).
	pub show_builder_values: bool,
	
	/// Configures whether artifacts should be only visualized by their type (`false`) or
	/// by their value (`true`).
	pub show_artifact_values: bool,
}

impl Default for VisgraphDocOptions {
	fn default() -> Self {
		VisgraphDocOptions {
			show_builder_values: false,
			show_artifact_values: true,
		}
	}
}

/// Debugger outputting Visgraph dot file.
///
/// **Notice: This struct is only available if the `diagnostics` feature has been activated**.
///
/// The Visgraph Doctor generates a DOT graph about the dependencies of
/// the builders and generated artifacts.
///
/// ## Example
///
/// ```no_run
/// use std::fs::File;
/// use daab::ArtifactCache;
/// use daab::diagnostics::{VisgraphDoc, VisgraphDocOptions};
///
/// let mut cache = ArtifactCache::new_with_doctor(
///     VisgraphDoc::new(
///         VisgraphDocOptions {
///             show_builder_values: false,
///             show_artifact_values: true,
///         },
///         File::create("test-graph.dot").unwrap()
///     )
/// );
///
/// //...
/// ```
///
pub struct VisgraphDoc<W: Write> {
	/// Output options
	opts: VisgraphDocOptions,
	/// Output Write
	output: W,
	/// Counts (generation, instance) of artifacts
	/// It is used to making each artifact unique.
	/// The generation increases whenever a artifact might be recreated
	/// i.e. after a call to `clear()` or `invalidate()`.
	count: (u64, u64),
}

impl<W: Write> VisgraphDoc<W> {
	/// Creates a new Visgraph Doctor
	///
	pub fn new(opts: VisgraphDocOptions,
		mut output: W) -> Self {
		
		writeln!(output, "strict digraph \"{:?}\" {{ graph [labeljust = l];", opts).unwrap();
		
		VisgraphDoc {
			opts,
			output,
			count: (0, 0),
		}
	}
	
	/// Strigify given builder entry.
	fn builder_str<'a>(&self, builder: &'a BuilderEntry) -> &'a str {
		if self.opts.show_builder_values {
			&builder.dbg_text
		} else {
			builder.type_name
		}
	}
}

impl<W: Write> Drop for VisgraphDoc<W> {
	fn drop(&mut self) {
		writeln!(self.output, "}}").unwrap();
	}
}

impl<W: Write> Doctor for VisgraphDoc<W> {
	fn resolve(&mut self, builder: &BuilderEntry, used: &BuilderEntry) {
	
		writeln!(self.output,
			r#"  "{:p}" [label = {:?}]"#,
			builder.value.builder,
			self.builder_str(builder)
		).unwrap();
		
		writeln!(self.output,
			r#"  "{:p}" [label = {:?}]"#,
			used.value.builder,
			self.builder_str(used)
		).unwrap();
		
		writeln!(self.output,
			r#"  "{:p}" -> "{:p}""#,
			builder.value.builder,
			used.value.builder
		).unwrap();
		
		self.output.flush().unwrap();
		
	}
	
	
	fn build(&mut self, builder: &BuilderEntry, artifact: &ArtifactEntry) {
		let count = self.count;
		
		writeln!(self.output,
			r#"  "{:p}" [label = {:?}]"#,
			builder.value.builder,
			self.builder_str(builder)
		).unwrap();
		
		let s = if self.opts.show_artifact_values {
			format!(" :\n{}", artifact.dbg_text)
		} else {
			"".into()
		};
		
		writeln!(self.output,
			r##"  "{0}.{1}-{2:p}" [label = "#{0}.{1} {3}{4}", shape = box]"##,
			count.0,
			count.1,
			artifact.value,
			artifact.type_name,
			s
		).unwrap();
			
		writeln!(self.output,
			r#"  "{:p}" -> "{}.{}-{:p}" [arrowhead = "none"]"#,
			builder.value.builder,
			count.0,
			count.1,
			artifact.value
		).unwrap();
		
		self.output.flush().unwrap();
			
		
		self.count.1 += 1;
		
	}
}

/// Default no-op `Doctor`.
///
/// **Notice: This struct is only available if the `diagnostics` feature has been activated**.
///
/// A no-op implementation of the `Doctor` i.e. a `Doctor` that does nothing. It is used as default `Doctor`,
/// i.e. if no actual `Doctor` is specified.
///
pub struct NoopDoctor;

impl Doctor for NoopDoctor {
	// Use default impl
}

impl Default for NoopDoctor {
	fn default() -> Self {
		NoopDoctor
	}
}




