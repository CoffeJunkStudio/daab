

use super::Doctor;
use super::BuilderHandle;
use super::ArtifactHandle;

use std::io::Write;


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
	fn builder_str<'a>(&self, builder: &'a BuilderHandle) -> &'a str {
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
	fn resolve(&mut self, builder: &BuilderHandle, used: &BuilderHandle) {
	
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
	
	
	fn build(&mut self, builder: &BuilderHandle, artifact: &ArtifactHandle) {
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



