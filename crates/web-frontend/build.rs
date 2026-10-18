use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let project_root = env::var("CARGO_MANIFEST_DIR")
        .ok()
        .and_then(|v| Path::new(&v).ancestors().nth(2).map(|p| p.to_path_buf()))
        .expect("Failed to find project root");

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");

    // Collect all source files
    let mut files: BTreeMap<String, String> = BTreeMap::new();
    collect_files(&project_root, &project_root, &mut files);

    // Serialize to JSON
    let json = serde_json::to_string(&files).expect("Failed to serialize files to JSON");

    // Write the Rust source file with the embedded JSON
    let dest_path = Path::new(&out_dir).join("source_files.rs");
    let json_bytes = json.as_bytes();
    fs::write(
        &dest_path,
        format!(
            "fn init_source_files() -> &'static str {{\n    static BYTES: &[u8] = &{:?};\n    std::str::from_utf8(BYTES).expect(\"valid utf8\")\n}}\n",
            json_bytes
        ),
    ).expect("Failed to write source_files.rs");

    println!("cargo:rerun-if-changed=.");
}

fn collect_files(project_root: &Path, current: &Path, files: &mut BTreeMap<String, String>) {
    let entries = match fs::read_dir(current) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let relative = path.strip_prefix(project_root).unwrap_or(&path);
        let relative_str = relative.to_string_lossy().to_string();

        // Skip directories and files we don't want
        if relative_str.starts_with("target/")
            || relative_str.starts_with(".git/")
            || relative_str.starts_with(".idea/")
            || relative_str.starts_with(".junie/")
            || relative_str == "Cargo.lock"
            || relative_str.contains("/target/")
            || relative_str.contains("/.git/")
            || relative_str.contains("/.idea/")
            || relative_str.contains("/.junie/")
        {
            continue;
        }

        // Skip large files (> 500KB) and likely binaries
        if let Ok(metadata) = fs::metadata(&path)
            && metadata.len() > 500_000
        {
            continue;
        }

        if path.is_dir() {
            collect_files(project_root, &path, files);
        } else if let Ok(content) = fs::read_to_string(&path) {
            // Skip binary files (contains null bytes)
            if content.contains('\0') {
                continue;
            }
            files.insert(relative_str, content);
        }
    }
}
