use std::collections::BTreeMap;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/source_files.rs"));

static FILES: OnceLock<BTreeMap<String, String>> = OnceLock::new();

fn init_files() -> &'static BTreeMap<String, String> {
    FILES.get_or_init(|| {
        let json_str = init_source_files();
        serde_json::from_str(json_str)
            .expect("Failed to parse embedded source files JSON")
    })
}

/// Returns all embedded files as a sorted map of path -> content
pub fn get_files() -> &'static BTreeMap<String, String> {
    init_files()
}

/// Check if a path is a directory (i.e., other files exist under it)
#[allow(dead_code)]
pub fn is_directory(path: &str) -> bool {
    let files = init_files();
    let prefix = if path.is_empty() { "" } else { &format!("{}/", path) };
    files.keys().any(|f| f.starts_with(prefix) && f != path)
}

/// List immediate children of a directory path
pub fn list_children(dir: &str) -> Vec<(String, bool)> {
    let files = init_files();
    let prefix = if dir.is_empty() { "" } else { &format!("{}/", dir) };
    let mut children: BTreeMap<String, bool> = BTreeMap::new();

    for file in files.keys() {
        if file == dir {
            continue;
        }
        if file.starts_with(prefix) {
            let remainder = &file[prefix.len()..];
            if let Some(slash) = remainder.find('/') {
                let subdir = &remainder[..slash];
                children.entry(subdir.to_string()).or_insert(true);
            } else if !remainder.is_empty() {
                children.entry(remainder.to_string()).or_insert(false);
            }
        }
    }

    children.into_iter().map(|(name, is_dir)| (name, is_dir)).collect()
}

/// Get the content of a file, or None if it doesn't exist
pub fn get_file_content(path: &str) -> Option<&'static str> {
    init_files().get(path).map(|s| s.as_str())
}
