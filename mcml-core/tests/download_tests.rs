use mcml_core::net::downloader::download_item::{DownloadItem, DownloadItemState};

#[test]
fn test_download_item_new() {
    let item = DownloadItem::new(
        "test_file.zip".to_string(),
        "https://example.com/test.zip".to_string(),
        "/downloads/test.zip".to_string(),
    );
    assert_eq!(item.name, "test_file.zip");
    assert_eq!(item.url, "https://example.com/test.zip");
    assert_eq!(item.local, "/downloads/test.zip");
    assert_eq!(item.state, DownloadItemState::Init);
    assert!(!item.overwrite);
    assert_eq!(item.all_size, 0);
    assert_eq!(item.now_size, 0);
    assert_eq!(item.error, 0);
    assert!(item.md5.is_none());
    assert!(item.sha1.is_none());
    assert!(item.sha256.is_none());
    assert!(item.later.is_none());
}

#[test]
fn test_download_item_progress_zero() {
    let item = DownloadItem::new(
        "empty".to_string(),
        "https://example.com/empty".to_string(),
        "/dev/null".to_string(),
    );
    assert_eq!(item.progress(), 0.0);
}

#[test]
fn test_download_item_progress_partial() {
    let mut item = DownloadItem::new(
        "partial".to_string(),
        "https://example.com/partial".to_string(),
        "/dev/null".to_string(),
    );
    item.all_size = 100;
    item.now_size = 50;
    assert!((item.progress() - 50.0).abs() < f64::EPSILON);
}

#[test]
fn test_download_item_progress_complete() {
    let mut item = DownloadItem::new(
        "complete".to_string(),
        "https://example.com/complete".to_string(),
        "/dev/null".to_string(),
    );
    item.all_size = 200;
    item.now_size = 200;
    assert!((item.progress() - 100.0).abs() < f64::EPSILON);
}

#[test]
fn test_download_item_with_md5() {
    let item = DownloadItem::new(
        "with_md5".to_string(),
        "https://example.com/file".to_string(),
        "/tmp/file".to_string(),
    )
    .with_md5("d41d8cd98f00b204e9800998ecf8427e".to_string());

    assert_eq!(
        item.md5,
        Some("d41d8cd98f00b204e9800998ecf8427e".to_string())
    );
}

#[test]
fn test_download_item_with_overwrite() {
    let item = DownloadItem::new(
        "overwrite_test".to_string(),
        "https://example.com/file".to_string(),
        "/tmp/file".to_string(),
    )
    .with_overwrite(true);

    assert!(item.overwrite);
}

#[test]
fn test_download_item_state_transitions() {
    let mut item = DownloadItem::new(
        "state_test".to_string(),
        "https://example.com/file".to_string(),
        "/tmp/file".to_string(),
    );

    assert_eq!(item.state, DownloadItemState::Init);

    item.state = DownloadItemState::Wait;
    assert_eq!(item.state, DownloadItemState::Wait);

    item.state = DownloadItemState::Download;
    assert_eq!(item.state, DownloadItemState::Download);

    item.state = DownloadItemState::Done;
    assert_eq!(item.state, DownloadItemState::Done);

    item.state = DownloadItemState::Error;
    assert_eq!(item.state, DownloadItemState::Error);
}

#[test]
fn test_download_item_state_debug() {
    let states = [
        DownloadItemState::Wait,
        DownloadItemState::Download,
        DownloadItemState::GetInfo,
        DownloadItemState::Pause,
        DownloadItemState::Init,
        DownloadItemState::Action,
        DownloadItemState::Done,
        DownloadItemState::Error,
    ];

    let debug_strs: Vec<String> = states.iter().map(|s| format!("{:?}", s)).collect();
    assert_eq!(debug_strs[0], "Wait");
    assert_eq!(debug_strs[1], "Download");
    assert_eq!(debug_strs[6], "Done");
    assert_eq!(debug_strs[7], "Error");
}

#[test]
fn test_download_item_error_count() {
    let mut item = DownloadItem::new(
        "error_count".to_string(),
        "https://example.com/file".to_string(),
        "/tmp/file".to_string(),
    );

    assert_eq!(item.error, 0);
    item.error += 1;
    assert_eq!(item.error, 1);
    item.error += 1;
    assert_eq!(item.error, 2);
}

#[test]
fn test_download_item_with_sha() {
    let item = DownloadItem::new(
        "sha_test".to_string(),
        "https://example.com/file".to_string(),
        "/tmp/file".to_string(),
    )
    .with_md5("md5_hash".to_string());

    // 单独设置 sha1 和 sha256
    let mut item2 = DownloadItem::new(
        "sha_test2".to_string(),
        "https://example.com/file2".to_string(),
        "/tmp/file2".to_string(),
    );
    item2.sha1 = Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string());
    item2.sha256 = Some(
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
    );

    assert_eq!(item.md5, Some("md5_hash".to_string()));
    assert_eq!(
        item2.sha1,
        Some("da39a3ee5e6b4b0d3255bfef95601890afd80709".to_string())
    );
    assert_eq!(
        item2.sha256,
        Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string())
    );
}
