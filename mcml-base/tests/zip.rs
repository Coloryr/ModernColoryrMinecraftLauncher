use std::{path::Path};

use mcml_base::archives::zip_runner::ZipProcess;

#[test]
fn zip() {
    let file = Path::new("tests");

    let runner = ZipProcess::new(None);

    let zip = file.join("test.zip");
    let dir = file.join("test_zip/");
    let filter = Some(vec![String::from("skip.text"), String::from("dir1/skip.text")]);
    let res = runner.zip(&zip, &dir, &dir, &filter);
    if let Err(err) = res {
        // println!("{}", err);
    }
}

#[test]
fn unzip() {
    let file = Path::new("tests");

    let runner = ZipProcess::new(None);

    let zip = file.join("test.zip");
    let dir = file.join("test_unzip/");
    let res = runner.unzip(&zip, &dir);
    if let Err(err) = res {
        // println!("{}", err);
    }
}