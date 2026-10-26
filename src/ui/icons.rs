use eframe::egui::{self, ColorImage, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::sync::Arc;

/// Icon manager for loading and caching SVG icons
pub struct IconManager {
    icons: HashMap<String, Arc<ColorImage>>,
    textures: HashMap<String, TextureHandle>,
}

impl std::fmt::Debug for IconManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IconManager")
            .field("icons", &self.icons.len())
            .field("textures", &self.textures.len())
            .finish()
    }
}

impl IconManager {
    pub fn new() -> Self {
        Self {
            icons: HashMap::new(),
            textures: HashMap::new(),
        }
    }

    /// Load an SVG icon from the assets directory
    pub fn load_svg(&mut self, name: &str, svg_data: &str, size: u32) -> Result<Arc<ColorImage>, String> {
        if let Some(cached) = self.icons.get(name) {
            return Ok(Arc::clone(cached));
        }

        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_str(svg_data, &opt)
            .map_err(|e| format!("Failed to parse SVG: {}", e))?;

        let mut pixmap = tiny_skia::Pixmap::new(size, size)
            .ok_or("Failed to create pixmap")?;

        let transform = tiny_skia::Transform::from_scale(
            size as f32 / tree.size().width(),
            size as f32 / tree.size().height(),
        );
        
        resvg::render(&tree, transform, &mut pixmap.as_mut());

        // Convert RGBA to ColorImage
        let pixels = pixmap.data();
        let mut rgba_pixels = Vec::with_capacity((size * size * 4) as usize);
        
        for chunk in pixels.chunks(4) {
            rgba_pixels.push(chunk[0]); // R
            rgba_pixels.push(chunk[1]); // G
            rgba_pixels.push(chunk[2]); // B
            rgba_pixels.push(chunk[3]); // A
        }

        let color_image = ColorImage::from_rgba_unmultiplied(
            [size as usize, size as usize],
            &rgba_pixels,
        );

        let arc_image = Arc::new(color_image);
        self.icons.insert(name.to_string(), Arc::clone(&arc_image));
        Ok(arc_image)
    }

    /// Get or create a texture for an icon
    pub fn get_texture(&mut self, ctx: &egui::Context, name: &str, svg_data: &str, size: u32) -> Result<TextureHandle, String> {
        if let Some(texture) = self.textures.get(name) {
            return Ok(texture.clone());
        }

        let color_image = self.load_svg(name, svg_data, size)?;
        let texture = ctx.load_texture(
            name,
            (*color_image).clone(),
            TextureOptions::LINEAR,
        );

        self.textures.insert(name.to_string(), texture.clone());
        Ok(texture)
    }
}

impl Default for IconManager {
    fn default() -> Self {
        Self::new()
    }
}

// SVG icon data embedded at compile time
pub const PLAY_SVG: &str = include_str!("../../assets/svg/play.svg");
pub const PAUSE_SVG: &str = include_str!("../../assets/svg/pause.svg");
pub const STOP_SVG: &str = include_str!("../../assets/svg/stop.svg");
pub const NEXT_SVG: &str = include_str!("../../assets/svg/next.svg");
pub const BACK_SVG: &str = include_str!("../../assets/svg/back.svg");
pub const MUTE_SVG: &str = include_str!("../../assets/svg/mute.svg");
pub const UNMUTE_SVG: &str = include_str!("../../assets/svg/unmute.svg");
