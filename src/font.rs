use freetype::Library;
use freetype::face::Face;
use std::collections::HashMap;
use std::rc::Rc;

pub struct RenderedGlyph {
    pub buffer: Vec<u8>,
    pub width: u32,
    pub rows: u32,
    pub bearing_x: f32,
    pub bearing_y: f32,
    pub advance: f32,
}

pub trait GlyphLoader {
    fn new(path: &str, size: u32) -> Self
    where
        Self: Sized;
    fn load(&self, key: usize) -> RenderedGlyph;
}

pub struct GlyphCache {
    cache: HashMap<usize, RenderedGlyph>,
    loader: Rc<GlyphLoader>,
}

impl GlyphCache {
    pub fn new<L: GlyphLoader + 'static>(loader: Rc<L>) -> Self {
        let mut cache = Self {
            cache: HashMap::new(),
            loader: loader,
        };

        // Prerender all visible ascii characters
        // TODO: change to `32..=127` when inclusive ranges make it to stable Rust
        for i in 32..128usize {
            cache.get(i);
        }

        cache
    }

    pub fn get(&mut self, key: usize) -> &RenderedGlyph {
        self.cache.entry(key).or_insert(self.loader.load(key))
    }
}

pub struct FreeTypeRasterizer {
    face: Face<'static>,
}

impl GlyphLoader for FreeTypeRasterizer {
    fn new(path: &str, size: u32) -> Self {
        let library = Library::init().unwrap();
        let face = library.new_face(path, 0).unwrap();

        face.set_pixel_sizes(0, size).unwrap();

        Self { face: face }
    }

    fn load(&self, key: usize) -> RenderedGlyph {
        self.face.load_char(key, ::freetype::face::RENDER).unwrap();
        let slot = self.face.glyph();

        RenderedGlyph {
            buffer: slot.bitmap().buffer().into(),
            width: slot.bitmap().width() as u32,
            rows: slot.bitmap().rows() as u32,
            bearing_x: slot.bitmap_left() as f32,
            bearing_y: slot.bitmap_top() as f32,
            // TODO: figure out why I need to divide by 2.0
            advance: slot.advance().x as f32 / 26.6 / 2.0,
        }
    }
}
