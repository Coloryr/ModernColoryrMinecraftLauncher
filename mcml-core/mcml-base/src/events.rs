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
