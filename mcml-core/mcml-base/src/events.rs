//! 通用事件系统
//!
//! 提供基于回调的发布-订阅事件机制，支持带参数和无参数两种模式。
//! 使用 `RwLock<HashMap<u64, Box<dyn Fn>>>` 存储回调，原子自增 ID 管理。

/// 通用事件
use std::{
    collections::HashMap,
    sync::{
        RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

/// 带参数通用事件
pub struct EventArgHandler<E> {
    handlers: RwLock<HashMap<u64, Box<dyn Fn(&E) + Send + Sync>>>,
    index: AtomicU64,
}

impl<E> EventArgHandler<E> {
    /// 创建事件处理器
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            index: AtomicU64::new(0),
        }
    }

    /// 添加事件处理器
    ///
    /// - `handler`: 回调函数
    pub fn add_handler<F>(&self, handler: F) -> u64
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = self.gen_next_id();
        self.handlers.write().unwrap().insert(id, Box::new(handler));

        id
    }

    /// 移除事件处理器
    ///
    /// - `id`: 处理器编号
    pub fn remove_handel(&self, id: u64) {
        self.handlers.write().unwrap().remove(&id);
    }

    /// 触发事件
    ///
    /// - `event`: 事件参数
    pub fn emit(&self, event: E) {
        for (_, handler) in self.handlers.read().unwrap().iter() {
            handler(&event);
        }
    }

    fn gen_next_id(&self) -> u64 {
        self.index.fetch_add(1, Ordering::SeqCst)
    }
}

/// 不带参数通用事件
pub struct EventHandler {
    handlers: RwLock<HashMap<u64, Box<dyn Fn() + Send + Sync>>>,
    index: AtomicU64,
}

impl EventHandler {
    /// 创建事件处理器
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            index: AtomicU64::new(0),
        }
    }

    /// 添加事件处理器
    ///
    /// - `handler`: 回调函数
    pub fn add_handler<F>(&self, handler: F) -> u64
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = self.gen_next_id();
        self.handlers.write().unwrap().insert(id, Box::new(handler));

        id
    }

    /// 移除事件处理器
    ///
    /// - `id`: 处理器编号
    pub fn remove_handle(&self, id: u64) {
        self.handlers.write().unwrap().remove(&id);
    }

    /// 触发事件
    pub fn emit(&self) {
        for (_, handler) in self.handlers.read().unwrap().iter() {
            handler();
        }
    }

    fn gen_next_id(&self) -> u64 {
        self.index.fetch_add(1, Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    /// 带参数事件的添加、触发、移除
    #[test]
    fn test_event_arg_handler() {
        let event: EventArgHandler<i32> = EventArgHandler::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = counter.clone();
        let id1 = event.add_handler(move |e: &i32| {
            // 收到 0 时不计数
            if *e != 0 {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });
        let c2 = counter.clone();
        let id2 = event.add_handler(move |_| {
            c2.fetch_add(1, Ordering::SeqCst);
        });
        assert_ne!(id1, id2, "每个处理器应有唯一 id");

        event.emit(1);
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        // 移除一个处理器后只触发另一个
        event.remove_handel(id1);
        event.emit(2);
        assert_eq!(counter.load(Ordering::SeqCst), 3);

        // 移除不存在的 id 不应 panic
        event.remove_handel(999);
    }

    /// 不带参数事件的添加、触发、移除
    #[test]
    fn test_event_handler() {
        let event = EventHandler::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let c = counter.clone();
        let id = event.add_handler(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });

        event.emit();
        event.emit();
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        event.remove_handle(id);
        event.emit();
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        // 移除不存在的 id 不应 panic
        event.remove_handle(12345);
    }

    /// 多个事件实例互不影响
    #[test]
    fn test_event_instances_isolated() {
        let e1 = EventHandler::new();
        let e2 = EventHandler::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let c = counter.clone();
        e1.add_handler(move || {
            c.fetch_add(1, Ordering::SeqCst);
        });
        e1.emit();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        e2.emit();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
