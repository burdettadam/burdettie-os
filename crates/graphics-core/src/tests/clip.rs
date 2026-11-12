use super::*;

fn px(surface: &Surface, x: u32, y: u32) -> u32 {
    surface.pixels()[(y * surface.width() + x) as usize]
}

const BLACK_ARGB: u32 = 0xFF000000;
const RED_ARGB: u32 = 0xFFFF0000;
const BLUE_ARGB: u32 = 0xFF0000FF;

#[test]
fn clip_no_clip_fills_everywhere() {
    let mut surface = Surface::new(100, 100);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::RED);
    }
    assert_eq!(px(&surface, 0, 0), RED_ARGB);
    assert_eq!(px(&surface, 50, 50), RED_ARGB);
    assert_eq!(px(&surface, 99, 99), RED_ARGB);
}

#[test]
fn clip_push_restricts_drawing() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        r.push_clip(Rect::new(10.0, 10.0, 30.0, 30.0));
        r.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::RED);
    }
    // Inside clip: should be RED
    assert_eq!(px(&surface, 15, 15), RED_ARGB);
    assert_eq!(px(&surface, 25, 25), RED_ARGB);
    assert_eq!(px(&surface, 39, 39), RED_ARGB);
    // Outside clip: should remain BLACK
    assert_eq!(px(&surface, 0, 0), BLACK_ARGB);
    assert_eq!(px(&surface, 5, 5), BLACK_ARGB);
    assert_eq!(px(&surface, 50, 50), BLACK_ARGB);
    assert_eq!(px(&surface, 99, 99), BLACK_ARGB);
}

#[test]
fn clip_nested_clips_intersect() {
    let mut surface = Surface::new(120, 120);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        // clip1: x0=10,y0=10 to x1=90,y1=90
        r.push_clip(Rect::new(10.0, 10.0, 80.0, 80.0));
        // clip2: x0=30,y0=30 to x1=110,y1=110
        r.push_clip(Rect::new(30.0, 30.0, 80.0, 80.0));
        // Intersection: (30,30) to (90,90)
        r.fill_rect(Rect::new(0.0, 0.0, 120.0, 120.0), Color::RED);
    }
    // Inside intersection: should be RED
    assert_eq!(px(&surface, 50, 50), RED_ARGB);
    assert_eq!(px(&surface, 30, 30), RED_ARGB);
    assert_eq!(px(&surface, 89, 89), RED_ARGB);
    // Outside intersection but inside one clip: should be BLACK
    assert_eq!(px(&surface, 15, 15), BLACK_ARGB); // inside clip1 only
    assert_eq!(px(&surface, 95, 95), BLACK_ARGB); // inside clip2 only
                                                  // Outside both clips
    assert_eq!(px(&surface, 5, 5), BLACK_ARGB);
}

#[test]
fn clip_pop_restores() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        // Push clip and fill — should be clipped
        r.push_clip(Rect::new(10.0, 10.0, 30.0, 30.0));
        r.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::RED);
        r.pop_clip();
        // Fill again — should be unclipped
        r.fill_rect(Rect::new(80.0, 80.0, 20.0, 20.0), Color::BLUE);
    }
    // The RED fill was clipped to (10,10)-(40,40)
    assert_eq!(px(&surface, 15, 15), RED_ARGB);
    assert_eq!(px(&surface, 5, 5), BLACK_ARGB);
    // The BLUE fill after pop was NOT clipped
    assert_eq!(px(&surface, 90, 90), BLUE_ARGB);
    assert_eq!(px(&surface, 85, 85), BLUE_ARGB);
}

#[test]
fn clip_current_clip_none() {
    let mut surface = Surface::new(10, 10);
    let r = SoftwareRenderer::new(&mut surface);
    assert!(r.current_clip().is_none());
}

#[test]
fn clip_current_clip_some() {
    let mut surface = Surface::new(10, 10);
    let mut r = SoftwareRenderer::new(&mut surface);
    r.push_clip(Rect::new(1.0, 2.0, 3.0, 4.0));
    let clip = r.current_clip().expect("should have a clip");
    assert_eq!(clip.x, 1.0);
    assert_eq!(clip.y, 2.0);
    assert_eq!(clip.width, 3.0);
    assert_eq!(clip.height, 4.0);
}

#[test]
fn clip_degenerate_clip() {
    let mut surface = Surface::new(100, 100);
    surface.clear(Color::BLACK);
    {
        let mut r = SoftwareRenderer::new(&mut surface);
        // Two non-overlapping clips: (10,10,30,30) and (50,50,30,30)
        r.push_clip(Rect::new(10.0, 10.0, 30.0, 30.0));
        r.push_clip(Rect::new(50.0, 50.0, 30.0, 30.0));
        // Intersection is empty — fill should paint nothing
        r.fill_rect(Rect::new(0.0, 0.0, 100.0, 100.0), Color::RED);
    }
    // Everything should remain BLACK
    assert_eq!(px(&surface, 0, 0), BLACK_ARGB);
    assert_eq!(px(&surface, 20, 20), BLACK_ARGB);
    assert_eq!(px(&surface, 60, 60), BLACK_ARGB);
    assert_eq!(px(&surface, 99, 99), BLACK_ARGB);
}
