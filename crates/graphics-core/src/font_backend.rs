//! Owned font data parsed by Fontations, with the renderer's existing pixel scale.
//!
//! A size is the ascent-to-descent height, not an em size. Keep design units
//! until the final multiplication so fractional sizes retain their precision.
//! The standalone rasterizer consumes curves only; it does not parse font data.

use ab_glyph_rasterizer::{point, Point, Rasterizer};
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::{tables::kern::SubtableKind, TableProvider};
use skrifa::{FontRef, MetadataProvider};

/// A glyph index within one font face. Zero denotes the missing glyph.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct GlyphId(pub u16);

impl GlyphId {
    pub(crate) fn with_scale_and_position(self, size: f32, position: Point) -> PositionedGlyph {
        PositionedGlyph {
            id: self,
            size,
            position,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PositionedGlyph {
    id: GlyphId,
    size: f32,
    position: Point,
}

/// Validated immutable bytes. Borrowed parser handles never outlive these bytes.
pub(crate) struct FontData {
    data: Vec<u8>,
    index: u32,
    metrics: skrifa::metrics::Metrics,
}

impl FontData {
    pub(crate) fn new(data: &[u8], index: u32) -> Option<Self> {
        let font = FontRef::from_index(data, index).ok()?;
        // FontRef validates the directory, but individual tables are lazy. Do
        // not accept truncated payloads or fonts without usable layout tables.
        for record in font.table_directory.table_records() {
            let start = usize::try_from(record.offset()).ok()?;
            let length = usize::try_from(record.length()).ok()?;
            data.get(start..start.checked_add(length)?)?;
        }
        font.head().ok()?;
        font.maxp().ok()?;
        font.hhea().ok()?;
        font.hmtx().ok()?;
        font.cmap().ok()?;
        let metrics = font.metrics(Size::unscaled(), LocationRef::default());
        if metrics.units_per_em == 0
            || metrics.glyph_count == 0
            || metrics.ascent <= metrics.descent
        {
            return None;
        }
        Some(Self {
            data: data.to_vec(),
            index,
            metrics,
        })
    }

    fn font(&self) -> FontRef<'_> {
        // The constructor validated the immutable bytes and the collection index.
        FontRef::from_index(&self.data, self.index).expect("validated font directory")
    }

    pub(crate) fn glyph_id(&self, ch: char) -> GlyphId {
        GlyphId(
            self.font()
                .charmap()
                .map(ch)
                .and_then(|id| u16::try_from(id.to_u32()).ok())
                .filter(|id| u32::from(*id) < self.metrics.glyph_count)
                .unwrap_or(0),
        )
    }

    pub(crate) fn as_scaled(&self, size: f32) -> ScaledFont<'_> {
        ScaledFont {
            font: self,
            factor: size / (self.metrics.ascent - self.metrics.descent),
        }
    }

    pub(crate) fn outline_glyph(&self, glyph: PositionedGlyph) -> Option<OutlinedGlyph> {
        if !glyph.size.is_finite()
            || glyph.size <= 0.0
            || !glyph.position.x.is_finite()
            || !glyph.position.y.is_finite()
            || u32::from(glyph.id.0) >= self.metrics.glyph_count
        {
            return None;
        }
        let font = self.font();
        let id = skrifa::GlyphId::new(glyph.id.0 as u32);
        let bounds = font
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .bounds(id)?;
        if bounds.x_min >= bounds.x_max || bounds.y_min >= bounds.y_max {
            return None;
        }
        let mut pen = CurvePen::default();
        font.outline_glyphs()
            .get(id)?
            .draw(
                DrawSettings::unhinted(Size::unscaled(), LocationRef::default()),
                &mut pen,
            )
            .ok()?;
        pen.close();
        let factor = self.as_scaled(glyph.size).factor;
        // Round relative to the fractional origin; large integer translations
        // must not change the raster coverage at an identical subpixel offset.
        let edge = |value: f32, origin: f32, lower: bool| {
            let relative = value * factor + origin.fract();
            (if lower {
                relative.floor()
            } else {
                relative.ceil()
            }) + origin.trunc()
        };
        let bounds = PixelBounds {
            min: point(
                edge(bounds.x_min, glyph.position.x, true),
                edge(-bounds.y_max, glyph.position.y, true),
            ),
            max: point(
                edge(bounds.x_max, glyph.position.x, false),
                edge(-bounds.y_min, glyph.position.y, false),
            ),
        };
        let width = bounds.max.x - bounds.min.x;
        let height = bounds.max.y - bounds.min.y;
        // Bound the raster allocation for malformed inputs and unreasonable
        // caller sizes. The GPU atlas applies its own smaller placement limit.
        if !width.is_finite()
            || !height.is_finite()
            || width <= 0.0
            || height <= 0.0
            || width as f64 * height as f64 > 16_777_216.0
        {
            return None;
        }
        Some(OutlinedGlyph {
            bounds,
            curves: pen.curves,
            factor,
            position: glyph.position,
        })
    }
}

pub(crate) struct ScaledFont<'a> {
    font: &'a FontData,
    factor: f32,
}

impl ScaledFont<'_> {
    pub(crate) fn ascent(&self) -> f32 {
        self.font.metrics.ascent * self.factor
    }

    pub(crate) fn descent(&self) -> f32 {
        self.font.metrics.descent * self.factor
    }

    pub(crate) fn line_gap(&self) -> f32 {
        self.font.metrics.leading * self.factor
    }

    pub(crate) fn h_advance(&self, id: GlyphId) -> f32 {
        self.font
            .font()
            .glyph_metrics(Size::unscaled(), LocationRef::default())
            .advance_width(skrifa::GlyphId::new(id.0 as u32))
            .unwrap_or(0.0)
            * self.factor
    }

    pub(crate) fn kern(&self, left: GlyphId, right: GlyphId) -> f32 {
        let font = self.font.font();
        let Ok(kern) = font.kern() else { return 0.0 };
        let left = skrifa::GlyphId::new(left.0 as u32);
        let right = skrifa::GlyphId::new(right.0 as u32);
        // Preserve the legacy simple-layout contract: first horizontal,
        // non-variable pair value. Advanced shaping remains in cosmic-text.
        kern.subtables()
            .filter_map(Result::ok)
            .filter(|table| {
                table.is_horizontal() && !table.is_cross_stream() && !table.is_variable()
            })
            .find_map(|table| match table.kind().ok()? {
                SubtableKind::Format0(table) => table.kerning(left, right),
                SubtableKind::Format2(table) => table.kerning(left, right),
                SubtableKind::Format3(table) => table.kerning(left, right),
                _ => None,
            })
            .unwrap_or(0) as f32
            * self.factor
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PixelBounds {
    pub(crate) min: Point,
    pub(crate) max: Point,
}

pub(crate) struct OutlinedGlyph {
    bounds: PixelBounds,
    curves: Vec<Curve>,
    factor: f32,
    position: Point,
}

impl OutlinedGlyph {
    pub(crate) fn px_bounds(&self) -> PixelBounds {
        self.bounds
    }

    pub(crate) fn draw(&self, output: impl FnMut(u32, u32, f32)) {
        let width = (self.bounds.max.x - self.bounds.min.x) as usize;
        let height = (self.bounds.max.y - self.bounds.min.y) as usize;
        let mut rasterizer = Rasterizer::new(width, height);
        let transform = |p: Point| {
            point(
                p.x * self.factor + (self.position.x - self.bounds.min.x),
                -p.y * self.factor + (self.position.y - self.bounds.min.y),
            )
        };
        for curve in &self.curves {
            match *curve {
                Curve::Line(a, b) => rasterizer.draw_line(transform(a), transform(b)),
                Curve::Quad(a, b, c) => {
                    rasterizer.draw_quad(transform(a), transform(b), transform(c))
                }
                Curve::Cubic(a, b, c, d) => {
                    rasterizer.draw_cubic(transform(a), transform(b), transform(c), transform(d));
                }
            }
        }
        rasterizer.for_each_pixel_2d(output);
    }
}

enum Curve {
    Line(Point, Point),
    Quad(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

#[derive(Default)]
struct CurvePen {
    curves: Vec<Curve>,
    current: Point,
    start: Option<Point>,
}

impl OutlinePen for CurvePen {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.current = point(x, y);
        self.start = Some(self.current);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let next = point(x, y);
        self.curves.push(Curve::Line(self.current, next));
        self.current = next;
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let next = point(x, y);
        self.curves
            .push(Curve::Quad(self.current, point(cx, cy), next));
        self.current = next;
    }

    fn curve_to(&mut self, cx0: f32, cy0: f32, cx1: f32, cy1: f32, x: f32, y: f32) {
        let next = point(x, y);
        self.curves.push(Curve::Cubic(
            self.current,
            point(cx0, cy0),
            point(cx1, cy1),
            next,
        ));
        self.current = next;
    }

    fn close(&mut self) {
        if let Some(start) = self.start.take() {
            self.curves.push(Curve::Line(self.current, start));
            self.current = start;
        }
    }
}

#[cfg(test)]
mod tests;
