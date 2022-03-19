///! A helper class for dealing with static archives
use std::mem;
use std::path::{Path, PathBuf};
use std::str;

use liblumen_llvm::archives::*;
use liblumen_session::Options;
use liblumen_session::OutputType;

use super::{find_library, ArchiveBuilder};

pub const RLIB_BYTECODE_EXTENSION: &str = "bc.z";
pub const METADATA_FILENAME: &str = "lib.rmeta";
pub const RUST_CGU_EXT: &str = "rcgu";

struct ArchiveConfig<'a> {
    pub sess: &'a Options,
    pub dst: PathBuf,
    pub src: Option<PathBuf>,
    pub lib_search_paths: Vec<PathBuf>,
}

/// Helper for adding many files to an archive.
#[must_use = "must call build() to finish building the archive"]
pub struct LlvmArchiveBuilder<'a> {
    config: ArchiveConfig<'a>,
    removals: Vec<String>,
    additions: Vec<Addition>,
    should_update_symbols: bool,
    src_archive: Option<Option<OwnedArchive>>,
}

enum Addition {
    File {
        path: PathBuf,
        name_in_archive: String,
    },
    Archive {
        path: PathBuf,
        archive: OwnedArchive,
        skip: Box<dyn FnMut(&str) -> bool>,
    },
}

impl Addition {
    fn path(&self) -> &Path {
        match self {
            Self::File { path, .. } | Self::Archive { path, .. } => path,
        }
    }
}

fn is_relevant_child(c: &ArchiveMember<'_>) -> bool {
    match c.name() {
        Some(name) => {
            let name: &str = name.try_into().unwrap();
            !name.contains("SYMDEF")
        }
        None => false,
    }
}

fn archive_config<'a>(sess: &'a Options, output: &Path, input: Option<&Path>) -> ArchiveConfig<'a> {
    use crate::linker::link::archive_search_paths;
    ArchiveConfig {
        sess,
        dst: output.to_path_buf(),
        src: input.map(|p| p.to_path_buf()),
        lib_search_paths: archive_search_paths(sess),
    }
}

impl<'a> ArchiveBuilder<'a> for LlvmArchiveBuilder<'a> {
    /// Creates a new static archive, ready for modifying the archive specified
    /// by `config`.
    fn new(sess: &'a Options, output: &Path, input: Option<&Path>) -> LlvmArchiveBuilder<'a> {
        let config = archive_config(sess, output, input);
        LlvmArchiveBuilder {
            config,
            removals: Vec::new(),
            additions: Vec::new(),
            should_update_symbols: false,
            src_archive: None,
        }
    }

    /// Removes a file from this archive
    fn remove_file(&mut self, file: &str) {
        self.removals.push(file.to_string());
    }

    /// Lists all files in an archive
    fn src_files(&mut self) -> Vec<String> {
        if self.src_archive().is_none() {
            return Vec::new();
        }

        let archive = self.src_archive.as_ref().unwrap().as_ref().unwrap();

        archive
            .iter()
            .filter_map(|child| child.ok())
            .filter(is_relevant_child)
            .filter_map(|child| child.name())
            .filter(|name| !self.removals.iter().any(|x| name.eq(x)))
            .map(|name| name.to_string())
            .collect()
    }

    /// Adds all of the contents of a native library to this archive. This will
    /// search in the relevant locations for a library named `name`.
    fn add_native_library(&mut self, name: &str) {
        let location = find_library(name, &self.config.lib_search_paths, self.config.sess).unwrap();
        self.add_archive(&location, |_| false).unwrap_or_else(|e| {
            panic!(
                "failed to add native library {}: {}",
                location.to_string_lossy(),
                e
            );
        });
    }

    /// Adds all of the contents of the rlib at the specified path to this
    /// archive.
    ///
    /// This ignores adding the bytecode from the rlib, and if LTO is enabled
    /// then the object file also isn't added.
    fn add_rlib(
        &mut self,
        rlib: &Path,
        name: &str,
        lto: bool,
        skip_objects: bool,
    ) -> anyhow::Result<()> {
        // Ignoring obj file starting with the crate name
        // as simple comparison is not enough - there
        // might be also an extra name suffix
        let obj_start = name.to_owned();

        self.add_archive(rlib, move |fname: &str| {
            // Ignore bytecode/metadata files, no matter the name.
            if fname.ends_with(RLIB_BYTECODE_EXTENSION) || fname == METADATA_FILENAME {
                return true;
            }

            // Don't include Rust objects if LTO is enabled
            if lto && looks_like_rust_object_file(fname) {
                return true;
            }

            // Otherwise if this is *not* a rust object and we're skipping
            // objects then skip this file
            if skip_objects && (!fname.starts_with(&obj_start) || !fname.ends_with(".o")) {
                return true;
            }

            // ok, don't skip this
            return false;
        })
    }

    /// Adds an arbitrary file to this archive
    fn add_file(&mut self, file: &Path) {
        let name = file.file_name().unwrap().to_str().unwrap();
        self.additions.push(Addition::File {
            path: file.to_path_buf(),
            name_in_archive: name.to_owned(),
        });
    }

    /// Indicate that the next call to `build` should update all symbols in
    /// the archive (equivalent to running 'ar s' over it).
    fn update_symbols(&mut self) {
        self.should_update_symbols = true;
    }

    /// Combine the provided files, rlibs, and native libraries into a single
    /// `Archive`.
    fn build(mut self) {
        let kind = self
            .llvm_archive_kind()
            .unwrap_or_else(|kind| panic!("Don't know how to build archive of type: {}", kind));

        if let Err(e) = self.build_with_llvm(kind) {
            panic!("failed to build archive: {}", e);
        }
    }
}

impl<'a> LlvmArchiveBuilder<'a> {
    fn src_archive(&mut self) -> Option<&Archive> {
        if let Some(ref opt) = self.src_archive {
            return opt.as_deref();
        }
        let src = self.config.src.as_ref()?;
        let opt = self.src_archive.insert(Archive::open(src).ok());
        opt.as_deref()
    }

    fn add_archive<F>(&mut self, path: &Path, skip: F) -> anyhow::Result<()>
    where
        F: FnMut(&str) -> bool + 'static,
    {
        let archive = Archive::open(path)?;
        if self.additions.iter().any(|ar| ar.path() == path) {
            return Ok(());
        }
        self.additions.push(Addition::Archive {
            path: path.to_path_buf(),
            archive,
            skip: Box::new(skip),
        });
        Ok(())
    }

    fn llvm_archive_kind(&self) -> Result<ArchiveKind, &str> {
        let kind = &*self.config.sess.target.options.archive_format;
        kind.parse().map_err(|_| kind)
    }

    fn build_with_llvm(&mut self, kind: ArchiveKind) -> anyhow::Result<()> {
        let removals = mem::take(&mut self.removals);
        let mut additions = mem::take(&mut self.additions);
        let mut strings = Vec::new();
        let mut members = Vec::new();

        let dst = self.config.dst.clone();
        let should_update_symbols = self.should_update_symbols;

        if let Some(archive) = self.src_archive() {
            for child in archive.iter() {
                let child = child?;
                let child_name = match child.name() {
                    Some(s) => s,
                    None => continue,
                };
                if removals.iter().any(|r| child_name.eq(r)) {
                    continue;
                }

                members.push(NewArchiveMember::from_child(child_name, child));
                strings.push(child_name.to_string());
            }
        }
        for addition in &mut additions {
            match addition {
                Addition::File {
                    path,
                    name_in_archive,
                } => {
                    members.push(NewArchiveMember::from_path(name_in_archive.as_str(), path));
                    strings.push(path.display().to_string());
                    strings.push(name_in_archive.to_string());
                }
                Addition::Archive { archive, skip, .. } => {
                    for child in archive.iter() {
                        let child = child?;
                        if !is_relevant_child(&child) {
                            continue;
                        }
                        let child_name = child.name().unwrap();
                        if skip(child_name.try_into().unwrap()) {
                            continue;
                        }

                        // It appears that LLVM's archive writer is a little
                        // buggy if the name we pass down isn't just the
                        // filename component, so chop that off here and
                        // pass it in.
                        //
                        // See LLVM bug 25877 for more info.
                        let child_name = child_name.to_path_lossy();
                        let child_name = child_name.file_name().unwrap();
                        members.push(NewArchiveMember::from_child(child_name, child));
                        strings.push(child_name.to_string_lossy().into_owned());
                    }
                }
            }
        }

        Archive::create(
            dst.as_path(),
            members.as_slice(),
            should_update_symbols,
            kind,
        )
    }
}

/// Checks if the given filename ends with the `.rcgu.o` extension that `rustc`
/// uses for the object files it generates.
fn looks_like_rust_object_file(filename: &str) -> bool {
    let path = Path::new(filename);
    let ext = path.extension().and_then(|s| s.to_str());
    if ext != Some(OutputType::Object.extension()) {
        // The file name does not end with ".o", so it can't be an object file.
        return false;
    }

    // Strip the ".o" at the end
    let ext2 = path
        .file_stem()
        .and_then(|s| Path::new(s).extension())
        .and_then(|s| s.to_str());

    // Check if the "inner" extension
    ext2 == Some(RUST_CGU_EXT)
}
