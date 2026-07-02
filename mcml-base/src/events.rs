use std::{
    collections::HashMap,
    sync::{
        RwLock,
        atomic::{AtomicU64, Ordering},
    },
};

pub struct EventArgHandler<E> {
    handlers: RwLock<HashMap<u64, Box<dyn Fn(&E) + Send + Sync>>>,
    index: AtomicU64,
}

impl<E> EventArgHandler<E> {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            index: AtomicU64::new(0),
        }
    }

    pub fn add_handler<F>(&self, handler: F) -> u64
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let id = self.gen_next_id();
        self.handlers.write().unwrap().insert(id, Box::new(handler));

        id
    }

    pub fn remove_handel(&self, id: u64) {
        self.handlers.write().unwrap().remove(&id);
    }

    pub fn emit(&self, event: E) {
        for (_, handler) in self.handlers.read().unwrap().iter() {
            handler(&event);
        }
    }

    fn gen_next_id(&self) -> u64 {
        self.index.fetch_add(1, Ordering::SeqCst)
    }
}

pub struct EventHandler {
    handlers: RwLock<HashMap<u64, Box<dyn Fn() + Send + Sync>>>,
    index: AtomicU64,
}

impl EventHandler {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
            index: AtomicU64::new(0),
        }
    }

    pub fn add_handler<F>(&self, handler: F) -> u64
    where
        F: Fn() + Send + Sync + 'static,
    {
        let id = self.gen_next_id();
        self.handlers.write().unwrap().insert(id, Box::new(handler));

        id
    }

    pub fn remove_handle(&self, id: u64) {
        self.handlers.write().unwrap().remove(&id);
    }

    pub fn emit(&self) {
        for (_, handler) in self.handlers.read().unwrap().iter() {
            handler();
        }
    }

    fn gen_next_id(&self) -> u64 {
        self.index.fetch_add(1, Ordering::SeqCst)
    }
}
