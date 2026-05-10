use chrono::{DateTime, FixedOffset, Local, Utc};

#[test]
fn test_time() {
    let dt = Local::now();
    let time: DateTime<FixedOffset> = dt.fixed_offset();

    println!("Local: {}", dt);
    println!("FixedOffset结果: {}", time);
}
