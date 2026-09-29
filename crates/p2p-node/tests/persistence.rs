use p2p_node::persistence;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
struct TestState {
    value: String,
    number: u64,
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "opendex-persistence-test-{}-{}",
        name,
        uuid::Uuid::new_v4()
    ))
}

#[test]
fn write_and_read_round_trip() {
    let path = temp_path("round-trip");

    let state = TestState {
        value: "hello".to_string(),
        number: 42,
    };

    persistence::write(&path, &state).unwrap();

    let loaded: TestState = persistence::read(&path).unwrap();

    assert_eq!(loaded, state);

    let _ = fs::remove_file(path);
}

#[test]
fn read_rejects_invalid_json() {
    let path = temp_path("invalid-json");

    fs::write(&path, b"{ definitely not valid json").unwrap();

    assert!(persistence::read::<TestState>(&path).is_err());

    let _ = fs::remove_file(path);
}

#[test]
fn write_bytes_round_trip() {
    let path = temp_path("bytes");

    let bytes = b"hello persistence";

    persistence::write_bytes(&path, bytes).unwrap();

    assert_eq!(persistence::read_bytes(&path).unwrap(), bytes);

    let _ = fs::remove_file(path);
}

#[test]
fn write_leaves_no_temporary_file_after_success() {
    let path = temp_path("cleanup");
    let parent = path.parent().unwrap();

    persistence::write(
        &path,
        &TestState {
            value: "ok".to_string(),
            number: 7,
        },
    )
    .unwrap();

    let file_name = path.file_name().unwrap().to_str().unwrap();

    let temporary_files: Vec<_> = fs::read_dir(parent)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_str().is_some_and(|name| {
                name.starts_with(&format!(".{}.", file_name)) && name.ends_with(".tmp")
            })
        })
        .collect();

    assert!(
        temporary_files.is_empty(),
        "temporary persistence files remain: {:?}",
        temporary_files
    );

    let _ = fs::remove_file(path);
}
