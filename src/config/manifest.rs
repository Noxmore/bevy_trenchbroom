use std::path::Path;

use bevy::platform::collections::HashSet;

use super::TrenchBroomConfig;

/// Static manifest of a directory tree. Paths are stored as `&str`s as `Path::new(...)` isn't stabilized as const at the time of writing.
#[derive(Debug, Clone, Copy)]
pub enum ManifestEntry {
	Directory { path: &'static str, children: &'static [ManifestEntry] },
	File { path: &'static str },
}
impl ManifestEntry {
	pub const fn path(&self) -> &'static str {
		match self {
			Self::Directory { path, .. } => path,
			Self::File { path } => path,
		}
	}
}

/// Contains a static manifest of a directory tree along with the set of files it contains for easy lookups.
#[derive(Debug, Clone)]
pub struct Manifest {
	entry: ManifestEntry,
	file_set: HashSet<&'static Path>,
}
impl Manifest {
	pub fn new(entry: ManifestEntry) -> Self {
		let mut file_set = HashSet::new();

		Self::populate_file_set(&mut file_set, &entry);

		Self { entry, file_set }
	}

	fn populate_file_set(file_set: &mut HashSet<&'static Path>, entry: &ManifestEntry) {
		match entry {
			ManifestEntry::Directory { children, .. } => {
				for child_entry in *children {
					Self::populate_file_set(file_set, child_entry);
				}
			}
			ManifestEntry::File { path } => {
				file_set.insert(Path::new(*path));
			}
		}
	}

	pub fn root(&self) -> &ManifestEntry {
		&self.entry
	}

	pub fn file_set(&self) -> &HashSet<&'static Path> {
		&self.file_set
	}
}

impl TrenchBroomConfig {
	/// Provides an compile-time manifest of all assets that is used to speed up loading in certain areas.
	/// Usually only provides a tangible boost for wasm builds.
	///
	/// Use with the `manifest!` macro passing in your assets folder. If you haven't changed it, it'll look like `.asset_manifest(manifest!("assets"))`.
	///
	/// NOTE: This will cause problems with adding/removing assets when hot-reloading is enabled. You might want to feature lock this to production builds.
	#[inline]
	pub fn asset_manifest(self, root: ManifestEntry) -> Self {
		Self { asset_manifest: Some(Manifest::new(root)), ..self }
	}
}
