use std::{env, fs, path::Path};

use quickraw::Export;

fn raw_path_from_env(var: &str) -> Option<String> {
    let path = env::var(var).ok()?;
    if !Path::new(&path).exists() {
        eprintln!("Skipping {var} test, file not found at '{path}'");
        return None;
    }
    Some(path)
}

fn assert_thumbnail_from(var: &str) {
    let Some(raw_path) = raw_path_from_env(var) else {
        return;
    };

    let output = std::env::temp_dir().join(format!("{var}.thumbnail.jpg"));
    print!(
        "Exporting thumbnail from {} to {} \n",
        raw_path,
        output.display()
    );

    Export::export_thumbnail_to_file(&raw_path, output.to_str().unwrap())
        .expect("thumbnail should be exported successfully");
    let data = fs::read(&output).expect("thumbnail output should be readable");
    assert!(!data.is_empty(), "thumbnail data must not be empty");
    // let _ = fs::remove_file(output);
}

#[test]
fn thumbnail_from_cr2() {
    assert_thumbnail_from("QUICKRAW_TEST_CANON_CR2");
}

#[test]
fn thumbnail_from_cr3() {
    assert_thumbnail_from("QUICKRAW_TEST_CANON_CR3");
}
