use std::sync::RwLock;

type StopHandler = Box<dyn Fn() + Send + Sync + 'static>;

static STOP_HANDLERS: RwLock<Vec<StopHandler>> = RwLock::new(Vec::new());

pub fn add_stop_handler<F>(handler: F)
where
    F: Fn() + Send + Sync + 'static,
{
    STOP_HANDLERS.write().unwrap().push(Box::new(handler));
}

pub fn invoke_stop() {
    for handler in STOP_HANDLERS.read().unwrap().iter() {
        handler();
    }
}
