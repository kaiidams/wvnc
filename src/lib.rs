mod app;
mod error;
mod framebuffer;
#[macro_use]
pub mod log;
mod audio;
mod d3des;
mod events;
mod io;
mod peer;
mod rfb;
mod task;

pub use self::error::*;
use crate::events::Event;

#[unsafe(no_mangle)]
pub extern "C" fn keyboard(down: bool, key: u32) {
    events::post(Event::Keyboard(down, key));
    task::run();
}

#[unsafe(no_mangle)]
pub extern "C" fn pointer(button: u8, x: u16, y: u16) {
    events::post(Event::Pointer(button, x, y));
    task::run();
}

#[unsafe(no_mangle)]
pub extern "C" fn clipboard() {
    events::post(Event::Clipboard());
    task::run();
}

#[unsafe(no_mangle)]
#[allow(clippy::uninit_vec)]
pub extern "C" fn alloc_buffer(len: usize) -> *mut u8 {
    let mut buf = Vec::with_capacity(len);
    unsafe {
        buf.set_len(len);
    }
    let ptr = buf.as_mut_ptr();
    std::mem::forget(buf);
    ptr
}

#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn socket_event(data: *mut u8, len: usize) {
    if data.is_null() {
        io::socket_onopen(len == 0);
    } else {
        let buf = unsafe { Vec::<u8>::from_raw_parts(data, len, len) };
        io::socket_onreceive(&buf);
    }
    task::run();
}

#[unsafe(no_mangle)]
pub extern "C" fn start() -> u32 {
    log::set_log_level(log::Level::Debug);
    task::init();
    events::init();
    io::init();
    task::spawn(async move {
        if let Result::Err(error) = app::appmain().await {
            log_error!("error={:?}", error);
        }
    });
    task::run();
    0
}
