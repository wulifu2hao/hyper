use std::ffi::c_void;
use std::ptr;

use libc::{c_int, size_t};
use hyper::body::{Body, Bytes, HttpBody as _};

use crate::{AssertSendSafe, HYPER_ITER_CONTINUE, task::Task};

pub struct hyper_body(pub(super) Body);

pub struct hyper_buf(pub(super) Bytes);

// ===== Body =====

type hyper_body_foreach_callback = extern "C" fn(*mut c_void, *const hyper_buf) -> c_int;

ffi_fn! {
    /// Free a `hyper_body *`.
    fn hyper_body_free(body: *mut hyper_body) {
        if body.is_null() {
            return;
        }

        drop(unsafe { Box::from_raw(body) });
    }
}

ffi_fn! {
    /// Return a task that will poll the body and execute the callback with each
    /// body chunk that is received.
    ///
    /// The `hyper_buf` pointer is only a borrowed reference, it cannot live outside
    /// the execution of the callback. You must make a copy to retain it.
    ///
    /// This will consume the `hyper_body *`, you shouldn't use it anymore or free it.
    fn hyper_body_foreach(body: *mut hyper_body, func: hyper_body_foreach_callback, userdata: *mut c_void) -> *mut Task {
        if body.is_null() {
            return ptr::null_mut();
        }

        let mut body = unsafe { Box::from_raw(body) };
        let userdata = AssertSendSafe(userdata);

        Box::into_raw(Task::boxed(async move {
            while let Some(item) = body.0.data().await {
                let chunk = item?;
                if HYPER_ITER_CONTINUE != func(userdata.0, &hyper_buf(chunk)) {
                    break;
                }
            }
            Ok(())
        }))
    }
}

// ===== Bytes =====


ffi_fn! {
    /// Get a pointer to the bytes in this buffer.
    ///
    /// This should be used in conjunction with `hyper_buf_len` to get the length
    /// of the bytes data.
    ///
    /// This pointer is borrowed data, and not valid once the `hyper_buf` is
    /// consumed/freed.
    fn hyper_buf_bytes(buf: *const hyper_buf) -> *const u8 {
        unsafe { (*buf).0.as_ptr() }
    }
}

ffi_fn! {
    /// Get the length of the bytes this buffer contains.
    fn hyper_buf_len(buf: *const hyper_buf) -> size_t {
        unsafe { (*buf).0.len() }
    }
}

ffi_fn! {
    /// Free this buffer.
    fn hyper_buf_free(buf: *mut hyper_buf) {
        drop(unsafe { Box::from_raw(buf) });
    }
}