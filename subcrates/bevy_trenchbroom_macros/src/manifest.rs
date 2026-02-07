use std::{path::Path, sync::Arc};

use crate::*;

pub fn manifest_impl(folder: String) -> TokenStream {
	let root: Arc<Path> = Path::new(&folder).into();
	generate_entry(&root, &root)
}

fn generate_entry(path: &Path, root: &Path) -> TokenStream {
	let metadata = match path.metadata() {
		Ok(metadata) => metadata,
		Err(err) => panic!("Failed to retrieve metadata for {}: {err}", path.display()),
	};

	let path_to_stringify = if !std::ptr::eq(path, root) {
		path.strip_prefix(root).unwrap_or(path)
	} else {
		path
	};

	let Some(path_str) = path_to_stringify.to_str() else {
		panic!("Failed to cleanly convert {} into a UTF-8 string", path.display());
	};

	if metadata.is_dir() {
		let entries = path
			.read_dir()
			.expect("Failed to read dir")
			.flatten()
			.map(|entry| generate_entry(&entry.path(), root))
			.collect::<Vec<TokenStream>>();

		quote! {
			::bevy_trenchbroom::config::ManifestEntry::Directory { path: #path_str, children: &[
				#(#entries,)*
			]}
		}
	} else {
		quote! {
			::bevy_trenchbroom::config::ManifestEntry::File { path: #path_str }
		}
	}
}
