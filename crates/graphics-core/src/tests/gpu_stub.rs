use crate::gpu_stub::GpuStubRenderer;
use crate::renderer::Renderer;
use crate::{create_renderer, Color, Font, Point, Rect, RendererBackend};

#[test]
fn gpu_renderer_creates() {
    let r = GpuStubRenderer::new(100, 100);
    assert_eq!(r.width(), 100);
    assert_eq!(r.height(), 100);
    assert_eq!(r.pixels().len(), 100 * 100);
}

#[test]
fn gpu_renderer_fill_rect() {
    let mut r = GpuStubRenderer::new(20, 20);
    let red = Color::new(1.0, 0.0, 0.0, 1.0);
    r.fill_rect(Rect::new(2.0, 2.0, 5.0, 5.0), red);
    // Pixel at (3, 3) should be red ARGB32: 0xFF_FF0000
    let idx = 3 * 20 + 3;
    assert_eq!(r.pixels()[idx], 0xFF_FF0000);
    // Pixel at (0, 0) should still be black (0)
    assert_eq!(r.pixels()[0], 0);
}

#[test]
fn gpu_renderer_draw_line() {
    let mut r = GpuStubRenderer::new(20, 20);
    let white = Color::new(1.0, 1.0, 1.0, 1.0);
    r.draw_line(Point::new(0.0, 10.0), Point::new(19.0, 10.0), white, 1.0);
    // At least one pixel on row 10 should be white
    let row_start = 10 * 20;
    let any_white = r.pixels()[row_start..row_start + 20]
        .iter()
        .any(|&p| p != 0);
    assert!(any_white, "expected at least one non-zero pixel on row 10");
}

#[test]
fn gpu_renderer_save_restore() {
    let mut r = GpuStubRenderer::new(10, 10);
    // save/restore are no-ops on the stub but must not panic
    r.save();
    r.fill_rect(
        Rect::new(0.0, 0.0, 5.0, 5.0),
        Color::new(0.0, 1.0, 0.0, 1.0),
    );
    r.restore();
}

#[test]
fn gpu_renderer_implements_renderer() {
    let mut r = GpuStubRenderer::new(50, 50);
    let dyn_r: &mut dyn Renderer = &mut r;
    dyn_r.clear(Color::new(0.0, 0.0, 0.0, 1.0));
    dyn_r.fill_rect(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        Color::new(1.0, 1.0, 1.0, 1.0),
    );
    // Compiles and runs — proves dyn Renderer compatibility
}

#[test]
fn create_renderer_software() {
    let r = create_renderer(RendererBackend::Software, 32, 32);
    // It returns a Box<dyn Renderer> — smoke-test clear
    let _ = r;
}

#[test]
fn create_renderer_gpu() {
    let mut r = create_renderer(RendererBackend::Gpu, 32, 32);
    r.clear(Color::new(0.5, 0.5, 0.5, 1.0));
}

#[test]
fn renderer_backend_enum() {
    assert_ne!(RendererBackend::Software, RendererBackend::Gpu);
    assert_eq!(RendererBackend::Software, RendererBackend::Software);
    assert_eq!(RendererBackend::Gpu, RendererBackend::Gpu);
    // Debug formatting
    let _ = format!("{:?}", RendererBackend::Software);
    let _ = format!("{:?}", RendererBackend::Gpu);
}
