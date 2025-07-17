use std::io::{Cursor, Write};
use std::panic::{self, PanicHookInfo};
use std::sync::OnceLock;

pub type PanicHandlerFn = extern "C" fn(data: *const u8, len: i32) -> !;

static EXTERNAL_PANIC_HANDLER: OnceLock<PanicHandlerFn> = OnceLock::new();

pub fn set_panic_handler(panic_handler_fn_ptr: PanicHandlerFn) {
    // there's nothing we can do if this fails, so we just ignore the result and
    // hope it succeeded
    let _ = EXTERNAL_PANIC_HANDLER.set(panic_handler_fn_ptr);
    panic::set_hook(Box::new(panic_hook));
}

fn panic_hook(info: &PanicHookInfo) {
    let mut buffer = [0_u8; 5000];
    let mut cursor = Cursor::new(buffer.as_mut());
    let _ = write!(cursor, "{info}"); // we can't panic if this fails, so we ignore the result

    #[cfg(feature = "backtrace")]
    {
        let _ = write!(
            cursor,
            "\nBacktrace:\n{}",
            std::backtrace::Backtrace::force_capture()
        );
    }

    let length = cursor.position();

    // we can't really do anything if the panic handler function isn't populated
    if let Some(panic_handler_fn) = EXTERNAL_PANIC_HANDLER.get() {
        panic_handler_fn(buffer.as_ptr(), length as i32);
    }
}
