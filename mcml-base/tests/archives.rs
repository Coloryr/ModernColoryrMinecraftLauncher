use std::path::Path;

use mcml_base::archives::{IArchive, r7z_runner::R7zProcess, zip_runner::ZipProcess};

#[test]
fn zip() {
    let file = Path::new("tests");

    let runner = ZipProcess::new(None);

    let zip = file.join("test.zip");
    let dir = file.join("test_compress/");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = runner.compress(&zip, &dir, Some(&dir), &filter);
    assert!(res.is_ok());
}

#[test]
fn unzip() {
    let file = Path::new("tests");

    let runner = ZipProcess::new(None);

    let zip = file.join("test.zip");
    let dir = file.join("test_unzip/");
    let res = runner.decompress(&zip, &dir);
    assert!(res.is_ok());
}

#[test]
fn r7z() {
    let file = Path::new("tests");

    let runner = R7zProcess::new(None);

    let zip = file.join("test.7z");
    let dir = file.join("test_compress/");
    let filter = Some(vec![
        String::from("skip.text"),
        String::from("dir1/skip.text"),
    ]);
    let res = runner.compress(&zip, &dir, Some(&dir), &filter);
    assert!(res.is_ok());
}

#[test]
fn unr7z() {
    let file = Path::new("tests");

    let runner = R7zProcess::new(None);

    let zip = file.join("test.7z");
    let dir = file.join("test_un7z/");
    let res = runner.decompress(&zip, &dir);
    assert!(res.is_ok());
}
