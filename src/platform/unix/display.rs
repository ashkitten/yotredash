extern crate glium;

// Glium

use glium::{glutin, Surface};
use glutin::{ContextBuilder, EventsLoop, WindowBuilder};
use glutin::os::unix::WindowExt;
use glutin::os::unix::x11::XConnection;
use glutin::os::unix::x11::ffi::{CWOverrideRedirect, Display, PropModeReplace, XSetWindowAttributes, XA_ATOM, XID};

// Clap

use clap::ArgMatches;

// Std

use std::sync::Arc;

pub trait DisplayExt {
    fn init(events_loop: &glutin::EventsLoop, args: &ArgMatches) -> Self;
    fn override_redirect(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID);
    fn lower_window(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID);
    fn desktop_window(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID);
    fn remap_window(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID);
}

impl DisplayExt for glium::Display {
    fn init(events_loop: &EventsLoop, args: &ArgMatches) -> Self {
        let width = args.value_of("width")
            .unwrap_or("640")
            .parse::<u32>()
            .unwrap();
        let height = args.value_of("height")
            .unwrap_or("400")
            .parse::<u32>()
            .unwrap();

        let window_builder = glutin::WindowBuilder::new().with_dimensions(width, height);

        let context = glutin::ContextBuilder::new().with_vsync(args.is_present("vsync"));

        let display = glium::Display::new(window_builder, context, &events_loop).unwrap();

        // Get info about our connection, display, and window
        let x_connection = display.gl_window().get_xlib_xconnection().unwrap();
        let x_display = display.gl_window().get_xlib_display().unwrap() as *mut Display;
        let x_window = display.gl_window().get_xlib_window().unwrap() as XID;

        if args.is_present("override-redirect") {
            // Set override-redirect attribute
            display.override_redirect(&x_connection, x_display, x_window);
            // After we set the override-redirect attribute, we need to remap the window for it to
            // take effect
            display.remap_window(&x_connection, x_display, x_window);
            // After remapping the window we need to set the size again
            display.gl_window().set_inner_size(width, height);
        }

        if args.is_present("lower-window") {
            display.lower_window(&x_connection, x_display, x_window);
        }

        if args.is_present("desktop") {
            display.desktop_window(&x_connection, x_display, x_window);
        }

        return display;
    }

    fn override_redirect(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID) {
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
                },
            );
        }
    }

    fn lower_window(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID) {
        unsafe {
            (x_connection.xlib.XLowerWindow)(x_display, x_window);
        }
    }

    fn desktop_window(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID) {
        let window_type_str = b"_NET_WM_WINDOW_TYPE\0".as_ptr();
        let window_type_desktop_str = b"_NET_WM_WINDOW_TYPE_DESKTOP\0".as_ptr();

        unsafe {
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
    }

    fn remap_window(&self, x_connection: &Arc<XConnection>, x_display: *mut Display, x_window: XID) {
        unsafe {
            // Remap the window so the override-redirect attribute can take effect
            // Unmap window
            (x_connection.xlib.XUnmapWindow)(x_display, x_window);
            // Sync (dunno why this is needed tbh, but it doesn't work without)
            (x_connection.xlib.XSync)(x_display, 0);
            // Remap window
            (x_connection.xlib.XMapWindow)(x_display, x_window);
        }
    }
}
