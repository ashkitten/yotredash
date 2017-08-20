extern crate glium;

use glium::glutin;
use glutin::EventsLoop;
use glutin::os::unix::WindowExt;
use glutin::os::unix::x11::XConnection;
use glutin::os::unix::x11::ffi::{CWOverrideRedirect, Display, PropModeReplace, XA_ATOM, XID, XSetWindowAttributes};
use std::sync::Arc;

use config::Config;

pub struct XContainer {
    connection: Arc<XConnection>,
    display: *mut Display,
    window: XID,
}

pub trait DisplayExt {
    fn init(events_loop: &glutin::EventsLoop, config: &Config) -> Self;
    fn override_redirect(&self, x: &XContainer);
    fn lower_window(&self, x: &XContainer);
    fn desktop_window(&self, x: &XContainer);
    fn remap_window(&self, x: &XContainer);
}

impl DisplayExt for glium::Display {
    fn init(events_loop: &EventsLoop, config: &Config) -> Self {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(config.buffers["__default__"].width, config.buffers["__default__"].height)
            .with_title("yotredash");

        let context = glutin::ContextBuilder::new().with_vsync(config.vsync);

        let display = glium::Display::new(window_builder, context, events_loop).unwrap();

        // Get info about our connection, display, and window
        let x = XContainer {
            connection: display.gl_window().get_xlib_xconnection().unwrap(),
            display: display.gl_window().get_xlib_display().unwrap() as *mut Display,
            window: display.gl_window().get_xlib_window().unwrap() as XID,
        };

        if config.platform_config.override_redirect {
            // Set override-redirect attribute
            display.override_redirect(&x);
            // After we set the override-redirect attribute, we need to remap the window for it to
            // take effect
            display.remap_window(&x);
            // After remapping the window we need to set the size again
            display.gl_window().set_inner_size(
                config.buffers["__default__"].width,
                config.buffers["__default__"].height,
            );
        }

        if config.platform_config.lower_window {
            display.lower_window(&x);
        }

        if config.platform_config.desktop {
            display.desktop_window(&x);
        }

        display
    }

    fn override_redirect(&self, x: &XContainer) {
        unsafe {
            // Change the override-redirect attribute
            (x.connection.xlib.XChangeWindowAttributes)(
                x.display,
                x.window,
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
    }

    fn lower_window(&self, x: &XContainer) {
        unsafe {
            (x.connection.xlib.XLowerWindow)(x.display, x.window);
        }
    }

    fn desktop_window(&self, x: &XContainer) {
        let window_type_str = b"_NET_WM_WINDOW_TYPE\0".as_ptr();
        let window_type_desktop_str = b"_NET_WM_WINDOW_TYPE_DESKTOP\0".as_ptr();

        unsafe {
            let window_type = (x.connection.xlib.XInternAtom)(x.display, window_type_str as *const i8, 0);
            let window_type_desktop =
                (x.connection.xlib.XInternAtom)(x.display, window_type_desktop_str as *const i8, 0);
            (x.connection.xlib.XChangeProperty)(
                x.display,
                x.window,
                window_type,
                XA_ATOM,
                32,
                PropModeReplace,
                &window_type_desktop as *const u64 as *const u8,
                1,
            );
        }
    }

    fn remap_window(&self, x: &XContainer) {
        unsafe {
            // Remap the window so the override-redirect attribute can take effect
            // Unmap window
            (x.connection.xlib.XUnmapWindow)(x.display, x.window);
            // Sync (dunno why this is needed tbh, but it doesn't work without)
            (x.connection.xlib.XSync)(x.display, 0);
            // Remap window
            (x.connection.xlib.XMapWindow)(x.display, x.window);
        }
    }
}
