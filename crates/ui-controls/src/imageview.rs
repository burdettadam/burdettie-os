//! ImageView control — displays an image with configurable content mode.

use graphics_core::{Color, ImageBuffer, Rect, Renderer};
use ui_view::layout::IntrinsicSize;
use ui_view::View;

/// How the image is scaled within the view's frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentMode {
    /// Scale to fill the entire frame (may distort).
    ScaleToFill,
    /// Scale to fit within frame, preserving aspect ratio.
    AspectFit,
    /// Scale to fill frame, preserving aspect ratio (may crop).
    AspectFill,
    /// Center without scaling.
    Center,
}

/// An image view control.
pub struct ImageView {
    view: View,
    image_pixels: Option<Vec<u32>>,
    image_width: u32,
    image_height: u32,
    content_mode: ContentMode,
    tint_color: Option<Color>,
}

impl ImageView {
    /// Create an image view with the given frame and no image.
    pub fn new(frame: Rect) -> Self {
        Self {
            view: View::new(frame),
            image_pixels: None,
            image_width: 0,
            image_height: 0,
            content_mode: ContentMode::ScaleToFill,
            tint_color: None,
        }
    }

    /// Set the image pixel data.
    pub fn set_image(&mut self, pixels: Vec<u32>, width: u32, height: u32) {
        self.image_pixels = Some(pixels);
        self.image_width = width;
        self.image_height = height;
        self.view.set_needs_display();
    }

    /// Remove the current image.
    pub fn clear_image(&mut self) {
        self.image_pixels = None;
        self.image_width = 0;
        self.image_height = 0;
        self.view.set_needs_display();
    }

    /// Whether an image is currently set.
    pub fn has_image(&self) -> bool {
        self.image_pixels.is_some()
    }

    /// The dimensions of the current image (width, height).
    pub fn image_size(&self) -> (u32, u32) {
        (self.image_width, self.image_height)
    }

    /// The current content mode.
    pub fn content_mode(&self) -> ContentMode {
        self.content_mode
    }

    /// Set the content mode.
    pub fn set_content_mode(&mut self, mode: ContentMode) {
        self.content_mode = mode;
        self.view.set_needs_display();
    }

    /// Set an optional tint color.
    pub fn set_tint_color(&mut self, color: Option<Color>) {
        self.tint_color = color;
        self.view.set_needs_display();
    }

    /// The current tint color.
    pub fn tint_color(&self) -> Option<Color> {
        self.tint_color
    }

    /// Compute the destination rect for the image based on content mode,
    /// frame, and image dimensions.
    pub fn image_draw_rect(&self) -> Rect {
        let frame = self.view.frame;
        if self.image_width == 0 || self.image_height == 0 {
            return Rect::new(frame.x, frame.y, 0.0, 0.0);
        }

        let iw = self.image_width as f64;
        let ih = self.image_height as f64;

        match self.content_mode {
            ContentMode::ScaleToFill => frame,
            ContentMode::AspectFit => {
                let scale = (frame.width / iw).min(frame.height / ih);
                let w = iw * scale;
                let h = ih * scale;
                Rect::new(
                    frame.x + (frame.width - w) / 2.0,
                    frame.y + (frame.height - h) / 2.0,
                    w,
                    h,
                )
            }
            ContentMode::AspectFill => {
                let scale = (frame.width / iw).max(frame.height / ih);
                let w = iw * scale;
                let h = ih * scale;
                Rect::new(
                    frame.x + (frame.width - w) / 2.0,
                    frame.y + (frame.height - h) / 2.0,
                    w,
                    h,
                )
            }
            ContentMode::Center => Rect::new(
                frame.x + (frame.width - iw) / 2.0,
                frame.y + (frame.height - ih) / 2.0,
                iw,
                ih,
            ),
        }
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable access to the underlying view.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Intrinsic content size based on the image dimensions.
    pub fn intrinsic_content_size(&self) -> IntrinsicSize {
        if self.image_pixels.is_some() {
            IntrinsicSize::new(self.image_width as f64, self.image_height as f64)
        } else {
            IntrinsicSize::new(0.0, 0.0)
        }
    }

    /// Draw the image view.
    pub fn draw(&mut self, renderer: &mut dyn Renderer) {
        if self.view.hidden {
            return;
        }

        // Draw background
        if self.view.background_color.a > 0.0 {
            renderer.fill_rect(self.view.frame, self.view.background_color);
        }

        // Draw image if present
        if let Some(ref pixels) = self.image_pixels {
            let dest = self.image_draw_rect();
            // Convert u32 ARGB pixels to RGBA8 bytes for ImageBuffer
            let mut rgba = Vec::with_capacity(pixels.len() * 4);
            for &px in pixels {
                rgba.push(((px >> 16) & 0xFF) as u8); // R
                rgba.push(((px >> 8) & 0xFF) as u8);  // G
                rgba.push((px & 0xFF) as u8);          // B
                rgba.push(((px >> 24) & 0xFF) as u8);  // A
            }
            if let Some(buf) =
                ImageBuffer::from_rgba(self.image_width, self.image_height, rgba)
            {
                renderer.draw_image(&buf, dest);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pixels(w: u32, h: u32) -> Vec<u32> {
        vec![0xFF_FF0000; (w * h) as usize] // opaque red ARGB
    }

    #[test]
    fn imageview_default() {
        let iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(!iv.has_image());
        assert_eq!(iv.content_mode(), ContentMode::ScaleToFill);
        assert_eq!(iv.image_size(), (0, 0));
    }

    #[test]
    fn imageview_set_image() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        iv.set_image(make_pixels(10, 10), 10, 10);
        assert!(iv.has_image());
        assert_eq!(iv.image_size(), (10, 10));
    }

    #[test]
    fn imageview_clear_image() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        iv.set_image(make_pixels(10, 10), 10, 10);
        assert!(iv.has_image());
        iv.clear_image();
        assert!(!iv.has_image());
        assert_eq!(iv.image_size(), (0, 0));
    }

    #[test]
    fn imageview_content_mode() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        for mode in [
            ContentMode::ScaleToFill,
            ContentMode::AspectFit,
            ContentMode::AspectFill,
            ContentMode::Center,
        ] {
            iv.set_content_mode(mode);
            assert_eq!(iv.content_mode(), mode);
        }
    }

    #[test]
    fn imageview_draw_rect_scale_to_fill() {
        let mut iv = ImageView::new(Rect::new(10.0, 20.0, 200.0, 100.0));
        iv.set_image(make_pixels(50, 50), 50, 50);
        iv.set_content_mode(ContentMode::ScaleToFill);
        let r = iv.image_draw_rect();
        assert_eq!(r.x, 10.0);
        assert_eq!(r.y, 20.0);
        assert_eq!(r.width, 200.0);
        assert_eq!(r.height, 100.0);
    }

    #[test]
    fn imageview_draw_rect_aspect_fit() {
        // 200x100 frame, 100x100 image → scale=1.0, centered 100x100
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        iv.set_image(make_pixels(100, 100), 100, 100);
        iv.set_content_mode(ContentMode::AspectFit);
        let r = iv.image_draw_rect();
        assert!((r.width - 100.0).abs() < 0.001);
        assert!((r.height - 100.0).abs() < 0.001);
        assert!((r.x - 50.0).abs() < 0.001); // centered horizontally
        assert!((r.y - 0.0).abs() < 0.001);
    }

    #[test]
    fn imageview_draw_rect_aspect_fill() {
        // 200x100 frame, 100x100 image → scale=2.0, 200x200 centered
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        iv.set_image(make_pixels(100, 100), 100, 100);
        iv.set_content_mode(ContentMode::AspectFill);
        let r = iv.image_draw_rect();
        assert!((r.width - 200.0).abs() < 0.001);
        assert!((r.height - 200.0).abs() < 0.001);
        assert!((r.x - 0.0).abs() < 0.001);
        assert!((r.y - (-50.0)).abs() < 0.001);
    }

    #[test]
    fn imageview_draw_rect_center() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 200.0, 100.0));
        iv.set_image(make_pixels(50, 30), 50, 30);
        iv.set_content_mode(ContentMode::Center);
        let r = iv.image_draw_rect();
        assert!((r.width - 50.0).abs() < 0.001);
        assert!((r.height - 30.0).abs() < 0.001);
        assert!((r.x - 75.0).abs() < 0.001);
        assert!((r.y - 35.0).abs() < 0.001);
    }

    #[test]
    fn imageview_intrinsic_no_image() {
        let iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        let s = iv.intrinsic_content_size();
        assert_eq!(s.width, Some(0.0));
        assert_eq!(s.height, Some(0.0));
    }

    #[test]
    fn imageview_intrinsic_with_image() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        iv.set_image(make_pixels(42, 17), 42, 17);
        let s = iv.intrinsic_content_size();
        assert_eq!(s.width, Some(42.0));
        assert_eq!(s.height, Some(17.0));
    }

    #[test]
    fn imageview_tint_color() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        assert!(iv.tint_color().is_none());
        iv.set_tint_color(Some(Color::RED));
        assert_eq!(iv.tint_color(), Some(Color::RED));
        iv.set_tint_color(None);
        assert!(iv.tint_color().is_none());
    }

    #[test]
    fn imageview_draw_no_panic() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        let mut surface = graphics_core::Surface::new(100, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        iv.draw(&mut renderer); // no image — should not panic
    }

    #[test]
    fn imageview_draw_with_image_no_panic() {
        let mut iv = ImageView::new(Rect::new(0.0, 0.0, 100.0, 100.0));
        iv.set_image(make_pixels(10, 10), 10, 10);
        let mut surface = graphics_core::Surface::new(100, 100);
        let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
        iv.draw(&mut renderer); // should not panic
    }

    #[test]
    fn imageview_view_access() {
        let mut iv = ImageView::new(Rect::new(5.0, 10.0, 80.0, 60.0));
        assert_eq!(iv.view().frame.x, 5.0);
        iv.view_mut().tag = 42;
        assert_eq!(iv.view().tag, 42);
    }
}
