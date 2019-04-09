use libc::{c_char, c_int, size_t};
use log::{info, warn, trace};
use std::ffi::{CString, CStr, VaList};

unsafe extern "C" fn alsa_error_handler(
    _file: *const c_char,
    _line: c_int,
    _function: *const c_char,
    _err: c_int,
    fmt: *const c_char,
    args: ...
) {
    const BUF_SIZE: usize = 1024;
    // safe to unwrap here
    let buf = CString::from_vec_unchecked(vec![0; BUF_SIZE]).into_raw();
    vsnprintf(buf, BUF_SIZE, fmt, args);
    let msg = CString::from_raw(buf);
    warn!(target: "yotredash::alsa", "{}", msg.to_string_lossy());
}

extern "C" fn jack_info_handler(msg: *const c_char) {
    let msg = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    info!(target: "yotredash::jack", "{}", msg);
}

extern "C" fn jack_error_handler(msg: *const c_char) {
    let msg = unsafe { CStr::from_ptr(msg).to_string_lossy() };
    warn!(target: "yotredash::jack", "{}", msg);
}

extern "C" {
    fn vsnprintf(s: *mut c_char, n: size_t, format: *const c_char, ap: VaList) -> c_int;
}

#[link(name = "asound")]
extern "C" {
    fn snd_lib_error_set_handler(
        handler: unsafe extern "C" fn(
            file: *const c_char,
            line: c_int,
            function: *const c_char,
            err: c_int,
            fmt: *const c_char,
            args: ...
        ),
    );
}

#[link(name = "jack")]
extern "C" {
    fn jack_set_info_function(handler: extern "C" fn(msg: *const c_char));
    fn jack_set_error_function(handler: extern "C" fn(msg: *const c_char));
}


pub fn setup_c_logging() {
    unsafe {
        snd_lib_error_set_handler(alsa_error_handler);
        jack_set_info_function(jack_info_handler);
        jack_set_error_function(jack_error_handler);
    }
    trace!("c logging setup");
}
