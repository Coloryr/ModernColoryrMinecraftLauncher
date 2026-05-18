use std::sync::RwLock;

type JvmChangeHandler = Box<dyn Fn() + Send + Sync + 'static>;

static JVM_HANDLERS: RwLock<Vec<JvmChangeHandler>> = RwLock::new(Vec::new());

pub fn add_jvm_handler<F>(handler: F)
where
    F: Fn() + Send + Sync + 'static,
{
    JVM_HANDLERS.write().unwrap().push(Box::new(handler));
}

pub fn invoke_jvm_change() {
    for handler in JVM_HANDLERS.read().unwrap().iter() {
        handler();
    }
}
