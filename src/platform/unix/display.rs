extern crate glium;

use glium::{glutin, Surface};
use Args;

use glutin::os::unix::WindowExt;
use glutin::os::unix::x11::ffi::{Display, XID, CWOverrideRedirect, XSetWindowAttributes, XA_ATOM, PropModeReplace};

pub trait DisplayExt {
    fn init(events_loop: &glutin::EventsLoop, args: &Args) -> Self;
    fn override_redirect(&self);
    fn lower_window(&self);
    fn desktop_window(&self);
}

impl DisplayExt for glium::Display {
    fn init(events_loop: &glutin::EventsLoop, args: &Args) -> Self {
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(args.width, args.height);

        let context = glutin::ContextBuilder::new();
        let display = glium::Display::new(window_builder, context, &events_loop).unwrap();

        if args.override_redirect {
            display.override_redirect();

            // After remapping the window we need to set the size again
            display.gl_window().set_inner_size(args.width, args.height);
        }

        if args.lower_window {
            display.lower_window();
        }

        if args.desktop {
            display.desktop_window();
        }

        return display;
    }

    fn override_redirect(&self) {
        // Get info about our connection, display, and window
        let x_connection = self.gl_window().get_xlib_xconnection().unwrap();
        let x_display = self.gl_window().get_xlib_display().unwrap() as *mut Display;
        let x_window = self.gl_window().get_xlib_window().unwrap() as XID;

        unsafe {
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
                }
            );
            // Remap the window so the override-redirect attribute can take effect
            (x_connection.xlib.XUnmapWindow)(x_display, x_window); // Unmap window
            (x_connection.xlib.XSync)(x_display, 0); // Sync (dunno why this is needed tbh, but it doesn't work without)
            (x_connection.xlib.XMapWindow)(x_display, x_window); // Remap window
        }
    }

    fn lower_window(&self) {
        // Get info about our connection, display, and window
        let x_connection = self.gl_window().get_xlib_xconnection().unwrap();
        let x_display = self.gl_window().get_xlib_display().unwrap() as *mut Display;
        let x_window = self.gl_window().get_xlib_window().unwrap() as XID;

        unsafe {
            (x_connection.xlib.XLowerWindow)(x_display, x_window);
        }
    }

    fn desktop_window(&self) {
        use std::ffi::CString;

        // Get info about our connection, display, and window
        let x_connection = self.gl_window().get_xlib_xconnection().unwrap();
        let x_display = self.gl_window().get_xlib_display().unwrap() as *mut Display;
        let x_window = self.gl_window().get_xlib_window().unwrap() as XID;

        unsafe {
            let window_type = (x_connection.xlib.XInternAtom)(x_display, CString::new("_NET_WM_WINDOW_TYPE").unwrap().as_ptr(), 0);
            let window_type_desktop = (x_connection.xlib.XInternAtom)(x_display, CString::new("_NET_WM_WINDOW_TYPE_DESKTOP").unwrap().as_ptr(), 0);
            (x_connection.xlib.XChangeProperty)(x_display, x_window, window_type, XA_ATOM, 32, PropModeReplace, &window_type_desktop as *const u64 as *const u8, 1);
        }
    }
}
