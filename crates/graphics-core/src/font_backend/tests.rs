use super::*;
use crate::font_manager::{
    DEJAVU_SANS, DEJAVU_SANS_BOLD, DEJAVU_SANS_BOLD_OBLIQUE, DEJAVU_SANS_MONO, DEJAVU_SANS_OBLIQUE,
};
use crate::FontFace;

fn bundled(file: &str) -> &'static [u8] {
    match file {
        "DejaVuSans.ttf" => DEJAVU_SANS,
        "DejaVuSans-Bold.ttf" => DEJAVU_SANS_BOLD,
        "DejaVuSansMono.ttf" => DEJAVU_SANS_MONO,
        "DejaVuSans-Oblique.ttf" => DEJAVU_SANS_OBLIQUE,
        "DejaVuSans-BoldOblique.ttf" => DEJAVU_SANS_BOLD_OBLIQUE,
        _ => panic!("unknown baseline font"),
    }
}

fn near(actual: f32, expected: &serde_json::Value, context: &str) {
    let expected = expected.as_f64().unwrap() as f32;
    assert!(
        (actual - expected).abs() <= 0.00001,
        "{context}: actual {actual}, expected {expected}"
    );
}

#[test]
fn bundled_metrics_outlines_and_pixels_match_pre_migration_baseline() {
    // Captured with the prior parser, before replacing its dependency. These
    // expected values are data, not recomputed through the new implementation.
    let baseline: serde_json::Value =
        serde_json::from_str(include_str!("data/dejavu-baseline.json")).unwrap();
    for face in baseline["faces"].as_array().unwrap() {
        let file = face["file"].as_str().unwrap();
        let loaded = FontFace::from_bytes(file, bundled(file)).unwrap();
        let font = loaded.inner();
        for sample in face["sizes"].as_array().unwrap() {
            let size = sample["size"].as_f64().unwrap() as f32;
            let scaled = font.as_scaled(size);
            for (i, actual) in [scaled.ascent(), scaled.descent(), scaled.line_gap()]
                .into_iter()
                .enumerate()
            {
                near(actual, &sample["metrics"][i], file);
            }
            near(
                scaled.kern(font.glyph_id('A'), font.glyph_id('V')),
                &sample["kern_av"],
                file,
            );
            for text in sample["texts"].as_array().unwrap() {
                near(
                    loaded.measure_text(text["text"].as_str().unwrap(), size as f64) as f32,
                    &text["width"],
                    file,
                );
            }
            for expected in sample["glyphs"].as_array().unwrap() {
                let codepoint = expected["codepoint"].as_u64().unwrap() as u32;
                let id = font.glyph_id(char::from_u32(codepoint).unwrap());
                assert_eq!(id.0 as u64, expected["id"].as_u64().unwrap(), "{file}");
                near(scaled.h_advance(id), &expected["advance"], file);
                let outlined =
                    font.outline_glyph(id.with_scale_and_position(size, point(0.25, 0.75)));
                let raster = &expected["raster"];
                if raster.is_null() {
                    assert!(outlined.is_none(), "{file} U+{codepoint:04X}");
                    continue;
                }
                let outlined = outlined.unwrap();
                let b = outlined.px_bounds();
                for (i, actual) in [b.min.x, b.min.y, b.max.x, b.max.y].into_iter().enumerate() {
                    near(actual, &raster["bounds"][i], file);
                }
                let width = (b.max.x - b.min.x) as usize;
                let mut actual = vec![0u8; width * (b.max.y - b.min.y) as usize];
                outlined.draw(|x, y, coverage| {
                    actual[y as usize * width + x as usize] =
                        (coverage * 255.0).clamp(0.0, 255.0) as u8;
                });
                let hex = raster["coverage_hex"].as_str().unwrap();
                let expected: Vec<u8> = (0..hex.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                    .collect();
                assert_eq!(actual.len(), expected.len());
                // Equivalent contour ordering can change the floating point
                // accumulation at an 8-bit rounding boundary by one level.
                let max_difference = actual
                    .iter()
                    .zip(&expected)
                    .map(|(a, b)| a.abs_diff(*b))
                    .max()
                    .unwrap_or(0);
                assert!(
                    max_difference <= 1,
                    "{file} size {size} U+{codepoint:04X}: pixel delta {max_difference}"
                );
            }
        }

        // Compare the unscaled outline's curve geometry independently of
        // rasterization. Ignore only zero-length contour-closing segments.
        for expected in face["outlines"].as_array().unwrap() {
            let Some(outline) = expected["outline"].as_object() else {
                continue;
            };
            let codepoint = expected["codepoint"].as_u64().unwrap() as u32;
            let id = font.glyph_id(char::from_u32(codepoint).unwrap());
            let outlined = font
                .outline_glyph(id.with_scale_and_position(24.5, point(0.0, 0.0)))
                .unwrap();
            let mut actual: Vec<Vec<f32>> = outlined
                .curves
                .iter()
                .map(|c| match *c {
                    Curve::Line(a, b) => vec![a.x, a.y, b.x, b.y],
                    Curve::Quad(a, b, c) => vec![a.x, a.y, b.x, b.y, c.x, c.y],
                    Curve::Cubic(a, b, c, d) => vec![a.x, a.y, b.x, b.y, c.x, c.y, d.x, d.y],
                })
                .collect();
            let mut expected: Vec<Vec<f32>> = outline["curves"]
                .as_array()
                .unwrap()
                .iter()
                .map(|c| {
                    c.as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_f64().unwrap() as f32)
                        .collect()
                })
                .collect();
            let nonempty = |c: &Vec<f32>| c.len() != 4 || c[0] != c[2] || c[1] != c[3];
            actual.retain(nonempty);
            expected.retain(nonempty);
            let compare = |a: &Vec<f32>, b: &Vec<f32>| {
                a.len().cmp(&b.len()).then_with(|| {
                    a.iter()
                        .zip(b)
                        .map(|(a, b)| a.total_cmp(b))
                        .find(|o| !o.is_eq())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            };
            actual.sort_by(compare);
            expected.sort_by(compare);
            assert_eq!(actual, expected, "{file} outline U+{codepoint:04X}");
        }
    }
}

#[test]
fn malformed_fonts_and_invalid_glyph_requests_fail_without_panicking() {
    for length in [0, 4, 12, 100, DEJAVU_SANS.len() / 2] {
        assert!(FontFace::from_bytes("truncated", &DEJAVU_SANS[..length]).is_none());
    }
    let mut corrupted = DEJAVU_SANS.to_vec();
    // The first table directory record must not point outside the font.
    corrupted[20..24].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(FontFace::from_bytes("bad offset", &corrupted).is_none());
    let font = FontData::new(DEJAVU_SANS, 0).unwrap();
    for size in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::MAX] {
        assert!(font
            .outline_glyph(
                font.glyph_id('A')
                    .with_scale_and_position(size, point(0.0, 0.0))
            )
            .is_none());
    }
    assert!(font
        .outline_glyph(GlyphId(u16::MAX).with_scale_and_position(16.0, point(0.0, 0.0)))
        .is_none());
    assert!(font
        .outline_glyph(
            font.glyph_id('A')
                .with_scale_and_position(16.0, point(f32::NAN, 0.0))
        )
        .is_none());
}

#[test]
fn collection_indices_select_distinct_faces_and_reject_out_of_range() {
    // Source-owned test collection assembled from the already bundled fonts.
    // TTC offsets are absolute, so relocate every SFNT table directory record.
    let mut collection = b"ttcf\0\x01\0\0\0\0\0\x02".to_vec();
    collection.resize(20, 0);
    for (index, data) in [DEJAVU_SANS, DEJAVU_SANS_BOLD].into_iter().enumerate() {
        let offset = collection.len() as u32;
        collection[12 + index * 4..16 + index * 4].copy_from_slice(&offset.to_be_bytes());
        let mut face = data.to_vec();
        let table_count = u16::from_be_bytes([face[4], face[5]]) as usize;
        for i in 0..table_count {
            let start = 12 + i * 16 + 8;
            let original = u32::from_be_bytes(face[start..start + 4].try_into().unwrap());
            face[start..start + 4].copy_from_slice(&(original + offset).to_be_bytes());
        }
        collection.extend(face);
        collection.resize(collection.len().next_multiple_of(4), 0);
    }
    let regular = FontFace::from_bytes_at_index("regular", &collection, 0).unwrap();
    let bold = FontFace::from_bytes_at_index("bold", &collection, 1).unwrap();
    assert_ne!(regular.char_width('A', 24.0), bold.char_width('A', 24.0));
    assert_eq!(
        bold.char_width('A', 24.0),
        FontFace::from_bytes("bold", DEJAVU_SANS_BOLD)
            .unwrap()
            .char_width('A', 24.0)
    );
    assert!(FontFace::from_bytes_at_index("bad index", &collection, 2).is_none());
    assert!(FontFace::from_bytes_at_index("bad index", DEJAVU_SANS, 1).is_none());
}

#[test]
fn maintained_shaper_preserves_embedded_font_layout_editing_and_hit_testing() {
    use cosmic_text::{
        Attrs, Buffer, Cursor, Edit, Editor, Family, FontSystem, Metrics, Selection, Shaping,
        SwashCache,
    };
    let mut db = cosmic_text::fontdb::Database::new();
    for data in [
        DEJAVU_SANS,
        DEJAVU_SANS_BOLD,
        DEJAVU_SANS_MONO,
        DEJAVU_SANS_OBLIQUE,
        DEJAVU_SANS_BOLD_OBLIQUE,
    ] {
        db.load_font_data(data.to_vec());
    }
    // The maintained font database must keep style/weight selection attached
    // to the intended embedded face, not silently fall back to regular.
    for (family, weight, style, expected) in [
        (
            "DejaVu Sans",
            cosmic_text::Weight::NORMAL,
            cosmic_text::Style::Normal,
            DEJAVU_SANS,
        ),
        (
            "DejaVu Sans",
            cosmic_text::Weight::BOLD,
            cosmic_text::Style::Normal,
            DEJAVU_SANS_BOLD,
        ),
        (
            "DejaVu Sans",
            cosmic_text::Weight::NORMAL,
            cosmic_text::Style::Italic,
            DEJAVU_SANS_OBLIQUE,
        ),
        (
            "DejaVu Sans",
            cosmic_text::Weight::BOLD,
            cosmic_text::Style::Italic,
            DEJAVU_SANS_BOLD_OBLIQUE,
        ),
        (
            "DejaVu Sans Mono",
            cosmic_text::Weight::NORMAL,
            cosmic_text::Style::Normal,
            DEJAVU_SANS_MONO,
        ),
    ] {
        let id = db
            .query(&cosmic_text::fontdb::Query {
                families: &[Family::Name(family)],
                weight,
                style,
                ..Default::default()
            })
            .unwrap();
        db.with_face_data(id, |data, index| {
            assert_eq!(index, 0);
            assert_eq!(data, expected, "{family} {weight:?} {style:?}");
        })
        .unwrap();
    }
    // Deliberately omit host font discovery: both the old baseline and this
    // regression must select only fonts shipped and licensed by this project.
    let mut fs = FontSystem::new_with_locale_and_db("en-US".into(), db);
    let mut cache = SwashCache::new();
    let baseline: serde_json::Value =
        serde_json::from_str(include_str!("data/shaping-baseline.json")).unwrap();
    for case in baseline["cases"].as_array().unwrap() {
        let text = case["text"].as_str().unwrap();
        let mut buffer = Buffer::new(&mut fs, Metrics::new(17.25, 22.0));
        buffer.set_size(case["width"].as_f64().map(|n| n as f32), None);
        buffer.set_text(
            text,
            &Attrs::new().family(Family::Name("DejaVu Sans")),
            Shaping::Advanced,
            None,
        );
        buffer.shape_until_scroll(&mut fs, false);
        let expected_runs = case["runs"].as_array().unwrap();
        assert_eq!(buffer.layout_runs().count(), expected_runs.len(), "{text}");
        for (run, expected) in buffer.layout_runs().zip(expected_runs) {
            assert_eq!(run.rtl, expected["rtl"].as_bool().unwrap());
            near(run.line_w, &expected["width"], text);
            near(run.line_top, &expected["top"], text);
            let glyphs = expected["glyphs"].as_array().unwrap();
            assert_eq!(run.glyphs.len(), glyphs.len(), "{text}");
            for (glyph, expected) in run.glyphs.iter().zip(glyphs) {
                assert_ne!(glyph.glyph_id, 0, "embedded font must cover {text}");
                assert_eq!(glyph.glyph_id as u64, expected["id"].as_u64().unwrap());
                assert_eq!(glyph.start as u64, expected["start"].as_u64().unwrap());
                assert_eq!(glyph.end as u64, expected["end"].as_u64().unwrap());
                assert_eq!(glyph.level.is_rtl(), expected["rtl"].as_bool().unwrap());
                near(glyph.x, &expected["x"], text);
                near(glyph.y, &expected["y"], text);
                near(glyph.w, &expected["width"], text);
                if glyph.w > 0.0 {
                    let hit = buffer
                        .hit(
                            glyph.x + glyph.w * 0.25,
                            run.line_top + run.line_height * 0.5,
                        )
                        .unwrap();
                    assert_eq!(hit.line, run.line_i);
                    assert!(
                        (glyph.start..=glyph.end).contains(&hit.index),
                        "{text}: hit {} outside {}..{}",
                        hit.index,
                        glyph.start,
                        glyph.end
                    );
                    assert!(text.is_char_boundary(hit.index));
                }
            }
            assert!(run
                .highlight(Cursor::new(0, 0), Cursor::new(0, text.len()))
                .any(|(_, width)| width > 0.0));
        }
        let mut covered_pixels = 0;
        buffer.draw(
            &mut fs,
            &mut cache,
            cosmic_text::Color::rgb(255, 255, 255),
            |_, _, _, _, color| {
                if color.a() > 0 {
                    covered_pixels += 1;
                }
            },
        );
        assert!(covered_pixels > 0, "shaped text must rasterize: {text}");
        let mut editor = Editor::new(buffer);
        editor.set_cursor(Cursor::new(0, text.len()));
        editor.set_selection(Selection::Normal(Cursor::new(0, 0)));
        assert_eq!(editor.copy_selection().as_deref(), Some(text));
    }
}
