
//!
//! # Extensive debugging analysis module.
//!
//! **Notice: This module is only available if the `diagnostics` feature has been activated**.
//!


use std::any::Any;
use std::io::Write;
use std::cell::RefCell;
use std::fs::File;

use super::ArtifactPromise;
use super::BuilderEntry;
use super::ArtifactEntry;


pub trait Doctor {
	fn resolve(&self, builder: &BuilderEntry, used: &BuilderEntry);
	fn build(&self, builder: &BuilderEntry, artifact: &ArtifactEntry);
}


#[derive(Debug, Copy, Clone)]
pub struct VisgrapDocOptions {
	builder_values: bool,
	artifact_values: bool,
}

impl Default for VisgrapDocOptions {
	fn default() -> Self {
		VisgrapDocOptions {
			builder_values: false,
			artifact_values: true,
		}
	}
}

pub struct VisgraphDoc {
	opts: VisgrapDocOptions,
	output: File,
	count: RefCell<u64>,
}

impl VisgraphDoc {
	pub fn def() -> Self {
		Self::new(VisgrapDocOptions {
			builder_values: false,
			artifact_values: true,
		}, std::fs::OpenOptions::new().write(true).truncate(true).create(true).open("output").unwrap())
	}
	
	pub fn new(opts: VisgrapDocOptions,
		mut output: File) -> Self {
		
		writeln!(output, "strict digraph \"{:?}\" {{ graph [labeljust = l];", opts).unwrap();
		
		VisgraphDoc {
			opts,
			output,
			count: RefCell::new(0),
		}
	}
	
	fn builder_str<'a>(&self, builder: &'a BuilderEntry) -> &'a str {
		if self.opts.builder_values {
			&builder.dbg_text
		} else {
			builder.type_name
		}
	}
}

impl Drop for VisgraphDoc {
	fn drop(&mut self) {
		writeln!(self.output, "}}").unwrap();
	}
}

impl Doctor for VisgraphDoc {
	fn resolve(&self, builder: &BuilderEntry, used: &BuilderEntry) {
		let mut out = &self.output;
	
		writeln!(out,
			r#"  "{:p}" [label = {:?}]"#, builder.value.builder, self.builder_str(builder)
		).unwrap();
		
		writeln!(out,
			r#"  "{:p}" [label = {:?}]"#, used.value.builder, self.builder_str(used)
		).unwrap();
		
		writeln!(out,
			r#"  "{:p}" -> "{:p}""#, builder.value.builder, used.value.builder
		).unwrap();
		
		out.flush();
		
		/*
		match self.opts {
			VisgrapDocOptions::Debug =>
				writeln!(self.output,
					r#"  "{:?}" -> "{:?}""#, (builder), (used)),
			VisgrapDocOptions::Type =>
				writeln!(self.output,
					r#"  "{}" -> "{}""#, (builder.type_name), (used.type_name)),
			VisgrapDocOptions::Value =>
				writeln!(self.output,
					r#"  {:?} -> {:?}"#, (&builder.dbg_text), (&used.dbg_text)),
		}.unwrap();
		self.output.flush().unwrap();
		*/
	}
	
	
	fn build(&self, builder: &BuilderEntry, artifact: &ArtifactEntry) {
		let count = *self.count.borrow();
		let mut out = &self.output;
		
		writeln!(out,
			r#"  "{:p}" [label = {:?}]"#, builder.value.builder, self.builder_str(builder)
		).unwrap();
		
		let s = if self.opts.artifact_values {
			format!(" :\n{}", artifact.dbg_text)
		} else {
			"".into()
		};
		
		writeln!(out,
			r##"  "{0}-{1:p}" [label = "#{0} {2}{3}", shape = box]"##, count,(artifact.value), (artifact.type_name), s
		).unwrap();
			
		writeln!(out,
			r#"  "{:p}" -> "{}-{:p}" [arrowhead = "none"]"#, (builder.value.builder), count,(artifact.value)
		).unwrap();
			
		
		*self.count.borrow_mut() += 1;
		
		/*
		match self.opts {
			VisgrapDocOptions::Debug =>
				writeln!(self.output,
					r#""{:?}" -> "{:?}""#, (builder), (artifact)),
			VisgrapDocOptions::Type =>
				writeln!(self.output,
					r#""{}" -> "{}""#, (builder.type_name), (artifact.type_name)),
			VisgrapDocOptions::Value =>
				writeln!(self.output,
					r#""{}" -> "{}""#, (builder.dbg_text), (artifact.dbg_text)),
		}.unwrap();
		*/
	}
}


pub struct NoopDoctor;

impl Doctor for NoopDoctor {
	fn resolve(&self, builder: &BuilderEntry, used: &BuilderEntry) {
		// NOOP
	}
	
	fn build(&self, builder: &BuilderEntry, artifact: &ArtifactEntry) {
		// NOOP
	}
}




