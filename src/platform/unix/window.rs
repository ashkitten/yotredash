//! Contains functions to apply Unix-specific window attributes and properties

use std::sync::Arc;
use winit::Window;
use winit::os::unix::WindowExt;
use winit::os::unix::x11::XConnection;
use winit::os::unix::x11::ffi::{CWOverrideRedirect, Display, PropModeReplace,
                                XSetWindowAttributes, XA_ATOM, XID};

use config::Config;

/// Sets the override-redirect flag of a window
unsafe fn override_redirect(
    x_connection: &Arc<XConnection>,
    x_display: *mut Display,
    x_window: XID,
) {
    // Change the override-redirect attribute
    (x_connection.xlib.XChangeWindowAttributes)(
        x_display,
        x_window,
        CWOverrideRedirect,
        &mut XSetWindowAttributes {
            background_pixmap: 0,
            background_pixel: 0,
            border_pixmap: 0,
            border_pixel: 0,
            bit_gravity: 0,
            win_gravity: 0,
            backing_store: 0,
            backing_planes: 0,
            backing_pixel: 0,
            save_under: 0,
            event_mask: 0,
            do_not_propagate_mask: 0,
            override_redirect: 1,
            colormap: 0,
            cursor: 0,
        },
    );
}

/// Lowers a window to the back of the stack
unsafe fn lower_window(x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID) {
    (x_connection.xlib.XLowerWindow)(x_display, x_window);
}

/// Sets the `_NET_WM_WINDOW_TYPE` atom of a window to `_NET_WM_WINDOW_TYPE_DESKTOP`
unsafe fn desktop_window(x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID) {
    let window_type_str = b"_NET_WM_WINDOW_TYPE\0".as_ptr();
    let window_type_desktop_str = b"_NET_WM_WINDOW_TYPE_DESKTOP\0".as_ptr();

    let window_type = (x_connection.xlib.XInternAtom)(x_display, window_type_str as *const i8, 0);
    let window_type_desktop =
        (x_connection.xlib.XInternAtom)(x_display, window_type_desktop_str as *const i8, 0);
    (x_connection.xlib.XChangeProperty)(
        x_display,
        x_window,
        window_type,
        XA_ATOM,
        32,
        PropModeReplace,
        &window_type_desktop as *const u64 as *const u8,
        1,
    );
}

/// Unmaps a window and maps it again
unsafe fn remap_window(x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID) {
    // Remap the window so the override-redirect attribute can take effect
    // Unmap window
    (x_connection.xlib.XUnmapWindow)(x_display, x_window);
    // Sync (dunno why this is needed tbh, but it doesn't work without)
    (x_connection.xlib.XSync)(x_display, 0);
    // Remap window
    (x_connection.xlib.XMapWindow)(x_display, x_window);
}

/// Initializes an X11 window according to a configuration
pub fn init(window: &Window, config: &Config) {
    // Get info about our connection, display, and window
    let x_connection = window.get_xlib_xconnection().unwrap();
    let x_display = window.get_xlib_display().unwrap() as *mut Display;
    let x_window = window.get_xlib_window().unwrap() as XID;

    unsafe {
        if config.platform_config.override_redirect {
            // Set override-redirect attribute
            override_redirect(&x_connection, x_display, x_window);
            // After we set the override-redirect attribute, we need to remap the window for it to
            // take effect
            remap_window(&x_connection, x_display, x_window);
            // After remapping the window we need to set the size again
            window.set_inner_size(
                config.buffers["__default__"].width,
                config.buffers["__default__"].height,
            );
        }

        if config.platform_config.lower_window {
            lower_window(&x_connection, x_display, x_window);
        }

        if config.platform_config.desktop {
            desktop_window(&x_connection, x_display, x_window);
        }
    }
}
