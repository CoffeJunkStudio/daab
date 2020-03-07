

use super::Doctor;
use super::BuilderHandle;
use super::ArtifactHandle;
use crate::CanBase;

use std::io::Write;
use cfg_if::cfg_if;

/// Output options for [`TextualDoc`].
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
///[`TextualDoc`]: struct.TextualDoc.html
///
///
/// ## Features
///
/// By default, this `Doctor` prints the type names of the builders and artifacts encountered.
/// The stringification is done via `std::any::type_name()`. However, this usually
/// generates long names, therefore this carte has the **`tynm`** feature, which adds
/// the [`tynm`] crate and allows to abbreviate the type names configured by this
/// struct's `tynm_m_n` field. Also see the [tynm docs] for details about `m` and `n`.
///
///[`tynm`]: https://crates.io/crates/tynm
///[tynm docs]: https://docs.rs/tynm/
///
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct TextualDocOptions {
	/// Configures whether builders should be only visualized by their
	/// value (`true`) instead of by their type (`false`)
	/// .
	pub show_builder_values: bool,
	
	/// Configures whether artifacts should be only visualized by their
	/// value (`true`) instead of by their type (`false`)
	pub show_artifact_values: bool,
	
	/// Configures whether the pointer of artifacts and builders should be
	/// printed for better identification (`true`) or
	/// not for better readability (`false`).
	pub show_addresses: bool,
	
	/// Configures type name abbreviations according to `tynm`s `type_namemn()` function.
	///
	/// `None` specifies to use the normal `std::any::type_name()`, and is the
	/// fallback if the **`tynm`** feature is not activated.
	///
	/// See the [tynm docs] for details about how to specify `m` and `n`.
	///
	/// **Notice:** the **`tynm`** feature is required for this field to take effect.
	///
	///[tynm docs]: https://docs.rs/tynm/
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
/// use daab::rc::ArtifactCache;
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
/// Example output:
///
/// ```text
/// resolves BuilderSimpleNode -> BuilderLeaf
/// built #0.0  BuilderLeaf => Rc<Leaf>
/// built #0.1  BuilderSimpleNode => Rc<SimpleNode>
/// resolves BuilderSimpleNode -> BuilderLeaf
/// built #0.2  BuilderSimpleNode => Rc<SimpleNode>
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
			output,
			count: (0, 0),
		}
	}
	
	fn tynm(&self, ty: &str) -> String {
		cfg_if! {
			if #[cfg(feature = "tynm")] {
				if let Some((m, n)) = self.opts.tynm_m_n {
					use tynm::TypeName;
					
					let tn: TypeName = ty.into();
					
					tn.as_str_mn(m, n)
				} else {
					ty.to_string()
				}
			} else {
				ty.to_string()
			}
		}
	}
	
	/// Strigify given builder entry.
	fn builder_str<'a, BCan>(&self, builder: &'a BuilderHandle<BCan>) -> String {
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

impl<ArtCan: CanBase, BCan, W: Write> Doctor<ArtCan, BCan> for TextualDoc<W> {
	fn resolve(&mut self, builder: &BuilderHandle<BCan>, used: &BuilderHandle<BCan>) {
	
		let bs = self.builder_str(builder);
		let us = self.builder_str(used);
		
		if self.opts.show_addresses {
			writeln!(self.output(),
				r#"resolves [{:p}] {} -> [{:p}] {}"#,
				builder.value.id,
				bs,
				used.value.id,
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
	
	
	fn build(&mut self, builder: &BuilderHandle<BCan>, artifact: &ArtifactHandle<ArtCan>) {
		let count = self.count;
		
		let bs = self.builder_str(builder);
		if self.opts.show_addresses {
			write!(self.output(),
				r#"built #{}.{} [{:p}] {} => [{:p}] "#,
				count.0,
				count.1,
				builder.value.id,
				bs,
				artifact.value.as_ptr(),
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
	
	fn invalidate(&mut self, builder: &BuilderHandle<BCan>) {
		let count = self.count;
		
		write!(self.output(),
			r"Invalidates generation #{} targeting ",
			count.0,
		).unwrap();
		
		let bs = self.builder_str(builder);
		if self.opts.show_addresses {
			write!(self.output(),
				"[{:p}] {}",
				builder.value.id,
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



