use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use mcml_core::events::core_stop_event::core_stop_event;

#[test]
fn test_add_stop_handler() {
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();
    core_stop_event::add_stop_handler(move || {
        called_clone.store(true, Ordering::SeqCst);
    });

    // 验证 handler 被添加（通过 invoke 触发）
    core_stop_event::invoke_stop();
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_multiple_stop_handlers() {
    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter_clone = counter.clone();

    core_stop_event::add_stop_handler(move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    let counter2 = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let counter2_clone = counter2.clone();

    core_stop_event::add_stop_handler(move || {
        counter2_clone.fetch_add(1, Ordering::SeqCst);
    });

    core_stop_event::invoke_stop();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert_eq!(counter2.load(Ordering::SeqCst), 1);
}

#[test]
fn test_stop_handler_order() {
    let results = Arc::new(std::sync::Mutex::new(Vec::new()));
    let results_clone1 = results.clone();
    let results_clone2 = results.clone();

    core_stop_event::add_stop_handler(move || {
        results_clone1.lock().unwrap().push("first");
    });

    core_stop_event::add_stop_handler(move || {
        results_clone2.lock().unwrap().push("second");
    });

    core_stop_event::invoke_stop();

    let final_results = results.lock().unwrap();
    assert_eq!(final_results.len(), 2);
    assert_eq!(final_results[0], "first");
    assert_eq!(final_results[1], "second");
}

#[test]
fn test_stop_handler_with_side_effects() {
    let shared_data = Arc::new(std::sync::Mutex::new(String::new()));
    let data_clone = shared_data.clone();

    core_stop_event::add_stop_handler(move || {
        let mut data = data_clone.lock().unwrap();
        data.push_str("cleaned_up");
    });

    core_stop_event::invoke_stop();

    let data = shared_data.lock().unwrap();
    assert_eq!(*data, "cleaned_up");
}
