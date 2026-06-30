use std::sync::RwLock;

/// 通用事件处理器集合
/// F 为处理器类型，例如 `Box<dyn Fn() + Send + Sync + 'static>`
pub struct Events<F> {
    handlers: RwLock<Vec<F>>,
}

impl<F> Events<F> {
    /// 创建空的事件处理器集合
    pub const fn new() -> Self {
        Events {
            handlers: RwLock::new(Vec::new()),
        }
    }

    /// 添加一个事件处理器
    pub fn add(&self, handler: F) {
        self.handlers.write().unwrap().push(handler);
    }

    /// 遍历所有处理器并执行指定操作
    pub fn for_each(&self, f: impl Fn(&F)) {
        for handler in self.handlers.read().unwrap().iter() {
            f(handler);
        }
    }
}

impl<F: Fn()> Events<F> {
    /// 以无参方式调用所有处理器
    pub fn invoke(&self) {
        for handler in self.handlers.read().unwrap().iter() {
            handler();
        }
    }
}
