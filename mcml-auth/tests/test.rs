use chrono::{DateTime, FixedOffset, Local};
use mcml_auth::oauth;

#[test]
fn test_time() {
    let dt = Local::now();
    let time: DateTime<FixedOffset> = dt.fixed_offset();

    println!("Local: {}", dt);
    println!("FixedOffset结果: {}", time);
}

#[tokio::test]
#[ignore = "skip"]
async fn oauth_test() {
    let code = oauth::get_code().await.unwrap();
}
