

use super::CanBase;
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
///     show_builder_values: false,
///     show_artifact_values: true,
/// };
/// assert_eq!(opts, VisgraphDocOptions::default());
/// ```
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct VisgraphDocOptions {
	/// Configures whether builders should be only visualized by their
	/// value (`true`) instead of by their type (`false`)
	/// .
	pub show_builder_values: bool,
	
	/// Configures whether artifacts should be only visualized by their
	/// value (`true`) instead of by their type (`false`)
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
/// use daab::rc::Cache;
/// use daab::diagnostics::{VisgraphDoc, VisgraphDocOptions};
///
/// let mut cache = Cache::new_with_doctor(
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
/// Example output in [DOT format]:
///
/// ```text
/// strict digraph { graph [labeljust = l];
///   "0x7faf30003960" [label = "alloc::rc::Rc<daab::test::BuilderSimpleNode>"]
///   "0x7faf30005090" [label = "alloc::rc::Rc<daab::test::BuilderLeaf>"]
///   "0x7faf30003960" -> "0x7faf30005090"
///   "0x7faf30005090" [label = "alloc::rc::Rc<daab::test::BuilderLeaf>"]
///   "0.0-0x7faf30015710" [label = "#0.0 daab::test::Leaf :
/// Leaf {
///     id: 0,
/// }", shape = box]
///   "0x7faf30005090" -> "0.0-0x7faf30015710" [arrowhead = "none"]
/// }
/// ```
///
///[DOT format]: https://en.wikipedia.org/wiki/DOT_%28graph_description_language%29
///
pub struct VisgraphDoc<W: Write> {
	/// Output options
	opts: VisgraphDocOptions,
	
	/// Output Write
	output: Option<W>,
	
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
		
		writeln!(output, "strict digraph {{ graph [labeljust = l];").unwrap();
		
		VisgraphDoc {
			opts,
			output: Some(output),
			count: (0, 0),
		}
	}
	
	/// Strigify given builder entry.
	fn builder_str<'a, BCan>(&self, builder: &'a BuilderHandle<BCan>) -> &'a str {
		if self.opts.show_builder_values {
			&builder.dbg_text
		} else {
			builder.type_name
		}
	}
	
	fn output(&mut self) -> &mut W {
		self.output.as_mut().unwrap()
	}
	
	fn finish(&mut self) {
		writeln!(self.output(), "}}").unwrap();
	}
	
	/// Dismantles this struct and returns the inner `Write`.
	///
	pub fn into_inner(mut self) -> W {
		self.finish();
		self.output.take().unwrap()
	}
}

impl<W: Write> Drop for VisgraphDoc<W> {
	fn drop(&mut self) {
		if self.output.is_some() {
			self.finish();
		}
	}
}

impl<ArtCan: CanBase, BCan, W: Write> Doctor<ArtCan, BCan> for VisgraphDoc<W> {
	fn resolve(&mut self, builder: &BuilderHandle<BCan>, used: &BuilderHandle<BCan>) {

		let s = self.builder_str(builder);
		writeln!(self.output(),
			r#"  "{:p}" [label = {:?}]"#,
			builder.id(),
			s
		).unwrap();

		let s = self.builder_str(used);
		writeln!(self.output(),
			r#"  "{:p}" [label = {:?}]"#,
			used.id(),
			s
		).unwrap();

		writeln!(self.output(),
			r#"  "{:p}" -> "{:p}""#,
			builder.id(),
			used.id()
		).unwrap();

		self.output().flush().unwrap();

	}
	
	
	fn build(&mut self, builder: &BuilderHandle<BCan>, artifact: &ArtifactHandle<ArtCan>) {
		let count = self.count;
		
		let s = self.builder_str(builder);
		writeln!(self.output(),
			r#"  "{:p}" [label = {:?}]"#,
			builder.id(),
			s
		).unwrap();
		
		let s = if self.opts.show_artifact_values {
			format!(" :\n{}", artifact.dbg_text)
		} else {
			"".into()
		};
		
		writeln!(self.output(),
			r##"  "{0}.{1}-{2:p}" [label = "#{0}.{1} {3}{4}", shape = box]"##,
			count.0,
			count.1,
			artifact.value.can_as_ptr(),
			artifact.type_name,
			s
		).unwrap();
			
		writeln!(self.output(),
			r#"  "{:p}" -> "{}.{}-{:p}" [arrowhead = "none"]"#,
			builder.id(),
			count.0,
			count.1,
			artifact.value.can_as_ptr()
		).unwrap();
		
		self.output().flush().unwrap();
			
		
		self.count.1 += 1;
		
	}
	
	fn clear(&mut self) {
		// Generations inc
		self.count.0 += 1;
		self.count.1 = 0;
	}
	
	fn invalidate(&mut self, _builder: &BuilderHandle<BCan>) {
		// Generations inc
		self.count.0 += 1;
		self.count.1 = 0;
	}
}



