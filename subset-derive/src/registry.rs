use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use syn::ImplItemFn;

thread_local! {
    /// Rewritten methods from prior derive invocations in this compilation.
    /// Stored as token strings to avoid span lifetime issues across expansions.
    static DERIVED: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());

    /// Cached file contents (avoids re-reading disk on repeated scans).
    static FILE_CACHE: RefCell<HashMap<PathBuf, String>> = RefCell::new(HashMap::new());
}

fn key(type_name: &str, method_name: &str) -> String {
    format!("{}::{}", type_name, method_name)
}

/// Register a derived/rewritten method so downstream subsets can find it.
pub fn register(type_name: &str, method_name: &str, method: &ImplItemFn) {
    let s = quote::quote!(#method).to_string();
    DERIVED.with(|d| d.borrow_mut().insert(key(type_name, method_name), s));
}

/// Look up a method by source type and name.
/// `target_struct` is the subset struct name — used to prefer same-file definitions
/// when multiple files define `impl <source_type>`.
pub fn get(target_struct: &str, source_type: &str, method_name: &str) -> Option<ImplItemFn> {
    // Check derived (rewritten) methods first — handles the derivative case
    // where a prior derive already produced this method.
    let derived = DERIVED.with(|d| d.borrow().get(&key(source_type, method_name)).cloned());
    if let Some(s) = derived {
        return syn::parse_str::<ImplItemFn>(&s).ok();
    }

    // Scan source files.
    find_in_files(target_struct, source_type, method_name)
}

/// Walk .rs files, find the method. Prefer the file that also contains `target_struct`.
fn find_in_files(target_struct: &str, source_type: &str, method_name: &str) -> Option<ImplItemFn> {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").ok()?;
    let mut rs_files = Vec::new();
    collect_rs_files(Path::new(&manifest_dir), &mut rs_files);

    let mut fallback: Option<String> = None;

    for path in &rs_files {
        let contents = read_cached(path);
        let file = match syn::parse_file(&contents) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let has_target_struct = file.items.iter().any(|item| {
            matches!(item, syn::Item::Struct(s) if s.ident == target_struct)
        });

        if let Some(method) = find_method_in_file(&file, source_type, method_name) {
            let method_str = quote::quote!(#method).to_string();
            if has_target_struct {
                // Same file as the subset struct — best match.
                return syn::parse_str::<ImplItemFn>(&method_str).ok();
            }
            if fallback.is_none() {
                fallback = Some(method_str);
            }
        }
    }

    fallback.and_then(|s| syn::parse_str::<ImplItemFn>(&s).ok())
}

fn find_method_in_file(
    file: &syn::File,
    source_type: &str,
    method_name: &str,
) -> Option<ImplItemFn> {
    for item in &file.items {
        let syn::Item::Impl(impl_block) = item else {
            continue;
        };
        if impl_block.trait_.is_some() {
            continue;
        }
        let type_name = match &*impl_block.self_ty {
            syn::Type::Path(tp) => tp.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        };
        if type_name.as_deref() != Some(source_type) {
            continue;
        }
        for impl_item in &impl_block.items {
            if let syn::ImplItem::Fn(method) = impl_item {
                if method.sig.ident == method_name {
                    return Some(method.clone());
                }
            }
        }
    }
    None
}

fn read_cached(path: &Path) -> String {
    FILE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(contents) = cache.get(path) {
            return contents.clone();
        }
        let contents = std::fs::read_to_string(path).unwrap_or_default();
        cache.insert(path.to_path_buf(), contents.clone());
        contents
    })
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with('.') || name == "target" {
                    continue;
                }
            }
            collect_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}
