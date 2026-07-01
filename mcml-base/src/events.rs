use std::sync::RwLock;

pub struct EventArgHandler<E> {
    handlers: RwLock<Vec<Box<dyn Fn(&E) + Send + Sync>>>,
}

impl<E> EventArgHandler<E> {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
        }
    }

    pub fn add_handler<F>(&self, handler: F)
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        self.handlers.write().unwrap().push(Box::new(handler));
    }

    pub fn emit(&self, event: E) {
        for handler in self.handlers.read().unwrap().iter() {
            handler(&event);
        }
    }
}

pub struct EventNormalHandler {
    handlers: RwLock<Vec<Box<dyn Fn() + Send + Sync>>>,
}

impl EventNormalHandler {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(Vec::new()),
        }
    }

    pub fn add_handler<F>(&self, handler: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.handlers.write().unwrap().push(Box::new(handler));
    }

    pub fn emit(&self) {
        for handler in self.handlers.read().unwrap().iter() {
            handler();
        }
    }
}
