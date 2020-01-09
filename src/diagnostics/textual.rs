

use super::Doctor;
use super::BuilderHandle;
use super::ArtifactHandle;

use std::io::Write;


/// Output options for `TextualDoc`.
///
/// **Notice: This struc is only available if the `diagnostics` feature has been activated**.
///
/// This struct contains outputting options for the `TextualDoc`.
///
/// It has a `Default` impl with the following value:
/// ```
/// # use daab::diagnostics::TextualDocOptions;
/// // Value of default()
/// let opts = TextualDocOptions {
///	    show_builder_values: false,
///	    show_artifact_values: false,
///	    show_addresses: false,
///	    tynm_m_n: Some((0,0)),
/// };
/// assert_eq!(opts, TextualDocOptions::default());
/// ```
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TextualDocOptions {
	/// Configures whether builders should be only visualized by their type (`false`) or
	/// by their value (`true`).
	pub show_builder_values: bool,
	
	/// Configures whether artifacts should be only visualized by their type (`false`) or
	/// by their value (`true`).
	pub show_artifact_values: bool,
	
	/// Configures whether the pointer of artifacts and builders should be printed for better identification (`true`) or
	/// not for better readability (`false`).
	pub show_addresses: bool,
	
	/// Configures type name abbreviations according to `tynm`s `type_namemn()` function.
	///
	pub tynm_m_n: Option<(usize, usize)>,
}

impl Default for TextualDocOptions {
	fn default() -> Self {
		TextualDocOptions {
			show_builder_values: false,
			show_artifact_values: false,
			show_addresses: false,
			tynm_m_n: Some((0,0)),
		}
	}
}

/// Debugger outputting human-readable text file e.g. on the terminal.
///
/// **Notice: This struct is only available if the `diagnostics` feature has been activated**.
///
/// The Textual Doctor generates writes the list of events in text form
/// to its output (e.g. `stdout`).
///
/// ## Example
///
/// ```no_run
/// use std::fs::File;
/// use daab::ArtifactCache;
/// use daab::diagnostics::{TextualDoc, TextualDocOptions};
/// use std::io::stdout;
///
/// let mut cache = ArtifactCache::new_with_doctor(
///     TextualDoc::new(
///         TextualDocOptions {
///             show_builder_values: false,
///             show_artifact_values: true,
///	            show_addresses: false,
///	            tynm_m_n: Some((0,0)),
///         },
///         stdout()
///     )
/// );
///
/// //...
/// ```
///
pub struct TextualDoc<W: Write> {
	/// Output options
	opts: TextualDocOptions,
	
	/// Output Write
	output: W,
	
	/// Counts (generation, instance) of artifacts
	/// It is used to making each artifact unique.
	/// The generation increases whenever a artifact might be recreated
	/// i.e. after a call to `clear()` or `invalidate()`.
	count: (u64, u64),
}

impl<W: Write> TextualDoc<W> {
	/// Creates a new Textual Doctor
	///
	pub fn new(opts: TextualDocOptions, output: W) -> Self {
		
		//writeln!(output, "strict digraph {{ graph [labeljust = l];").unwrap();
		
		TextualDoc {
			opts,
			output: output,
			count: (0, 0),
		}
	}
	
	fn tynm(&self, ty: &str) -> String {
		#[cfg(feature = "tynm")]
		{
			if let Some((m, n)) = self.opts.tynm_m_n {
				use tynm::TypeName;
				
				let tn: TypeName = ty.into();
				
				tn.as_str_mn(m, n)
			} else {
				ty.to_string()
			}
		}
		#[cfg(not(feature = "tynm"))]
		{
			ty.to_string()
		}
	}
	
	/// Strigify given builder entry.
	fn builder_str<'a>(&self, builder: &'a BuilderHandle) -> String {
		if self.opts.show_builder_values {
			builder.dbg_text.clone()
		} else {
			self.tynm(builder.type_name)
		}
	}
	
	/// Auxiliary to get the output by `&mut`.
	/// depricated
	fn output(&mut self) -> &mut W {
		&mut self.output
	}
	
	/// Dismantles this struct and returns the inner `Write`.
	///
	pub fn into_inner(self) -> W {
		self.output
	}
}

impl<W: Write> Doctor for TextualDoc<W> {
	fn resolve(&mut self, builder: &BuilderHandle, used: &BuilderHandle) {
	
		let bs = self.builder_str(builder);
		let us = self.builder_str(used);
		
		if self.opts.show_addresses {
			writeln!(self.output(),
				r#"resolves [{:p}] {} -> [{:p}] {}"#,
				builder.value.builder,
				bs,
				used.value.builder,
				us,
			).unwrap();
		} else {
			writeln!(self.output(),
				r#"resolves {} -> {}"#,
				bs,
				us,
			).unwrap();
		}
	}
	
	
	fn build(&mut self, builder: &BuilderHandle, artifact: &ArtifactHandle) {
		let count = self.count;
		
		let bs = self.builder_str(builder);
		if self.opts.show_addresses {
			write!(self.output(),
				r#"built #{}.{} [{:p}] {} => [{:p}] "#,
				count.0,
				count.1,
				builder.value.builder,
				bs,
				artifact.value,
			).unwrap();
		} else {
			write!(self.output(),
				r#"built #{}.{}  {} => "#,
				count.0,
				count.1,
				bs,
			).unwrap();
		}
		
		if self.opts.show_artifact_values {
			writeln!(self.output(),
				"{}",
				artifact.dbg_text,
			).unwrap();
		} else {
			let s = self.tynm(artifact.type_name);
			writeln!(self.output(),
				"{}",
				s,
			).unwrap();
		}
		
		self.output().flush().unwrap();
		
		self.count.1 += 1;
		
	}
	
	fn clear(&mut self) {
		let count = self.count;
		
		writeln!(self.output(),
			r"Clears generation #{}",
			count.0,
		).unwrap();
		
		// Generations inc
		self.count.0 += 1;
		self.count.1 = 0;
	}
	
	fn invalidate(&mut self, builder: &BuilderHandle) {
		let count = self.count;
		
		write!(self.output(),
			r"Invalidates generation #{} targeting ",
			count.0,
		).unwrap();
		
		let bs = self.builder_str(builder);
		if self.opts.show_addresses {
			write!(self.output(),
				"[{:p}] {}",
				builder.value.builder,
				bs,
			).unwrap();
		} else {
			write!(self.output(),
				"{}",
				bs,
			).unwrap();
		}
		
		// Generations inc
		self.count.0 += 1;
		self.count.1 = 0;
	}
}



