use cocoa::appkit::{NSApp, NSApplication, NSApplicationActivationPolicy};
use cocoa::base::nil;
use cocoa::foundation::NSAutoreleasePool;
use objc::{msg_send, sel, sel_impl};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

pub struct MacOSApp {
    running: Arc<AtomicBool>,
}

#[allow(non_snake_case)]
impl MacOSApp {
    pub fn new() -> Self {
        unsafe {
            let _pool = NSAutoreleasePool::new(nil);
            let app = NSApp();
            app.setActivationPolicy_(NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular);

        }

        Self {
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn get_running_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.running)
    }

    pub fn run(&self) {
        unsafe {
            let app = NSApp();
            let _: () = msg_send![app, run];
        }
    }

    pub fn terminate(&self) {
        self.running.store(false, Ordering::SeqCst);
        unsafe {
            let app = NSApp();
            let _: () = msg_send![app, terminate: nil];
        }
    }

    pub fn run_in_thread(&self) {
        std::thread::spawn(|| unsafe {
            cocoa::appkit::NSApp().run();
        });
    }
    
    
    
}
