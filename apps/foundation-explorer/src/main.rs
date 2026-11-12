//! Foundation Explorer — runtime introspection GUI.
//!
//! Registers all 40 Foundation classes into an ObjC runtime instance,
//! then lets the user browse classes and their methods.
//!
//! Run with: `cargo run -p foundation-explorer`

use graphics_core::{Color, Rect};
use ui_controls::{Button, Label};
use window_backend::WindowConfig;
use window_backend_darwin::GuiApp;

use objc_runtime::ObjcRuntime;

/// Register all 40 Foundation classes and return sorted class info.
fn build_class_data() -> Vec<ClassSummary> {
    let mut rt = ObjcRuntime::new();

    cf_lite::FoundationShims::register_nsobject_class(&mut rt);
    cf_lite::FoundationShims::register_nsstring_proper_class(&mut rt);
    cf_lite::FoundationShims::register_nsstring_class(&mut rt);
    cf_lite::FoundationShims::register_nsarray_class(&mut rt);
    cf_lite::FoundationShims::register_nsnumber_class(&mut rt);
    cf_lite::FoundationShims::register_nsdictionary_class(&mut rt);
    cf_lite::FoundationShims::register_nsmutablearray_class(&mut rt);
    cf_lite::FoundationShims::register_nsmutabledictionary_class(&mut rt);
    cf_lite::FoundationShims::register_nsmutablestring_class(&mut rt);
    cf_lite::FoundationShims::register_nsdata_class(&mut rt);
    cf_lite::FoundationShims::register_nsprocessinfo_class(&mut rt);
    cf_lite::FoundationShims::register_nserror_class(&mut rt);
    cf_lite::FoundationShims::register_nsbundle_class(&mut rt);
    cf_lite::FoundationShims::register_nsfilemanager_class(&mut rt);
    cf_lite::FoundationShims::register_nsexception_class(&mut rt);
    cf_lite::FoundationShims::register_nsenumerator_class(&mut rt);
    cf_lite::FoundationShims::register_nsdate_class(&mut rt);
    cf_lite::FoundationShims::register_nsuserdefaults_class(&mut rt);
    cf_lite::FoundationShims::register_nsset_class(&mut rt);
    cf_lite::FoundationShims::register_nsmutableset_class(&mut rt);
    cf_lite::FoundationShims::register_nsurl_class(&mut rt);
    cf_lite::FoundationShims::register_nsnull_class(&mut rt);
    cf_lite::FoundationShims::register_nsdateformatter_class(&mut rt);
    cf_lite::FoundationShims::register_nscharacterset_class(&mut rt);
    cf_lite::FoundationShims::register_nsnotificationcenter_class(&mut rt);
    cf_lite::FoundationShims::register_nsvalue_class(&mut rt);
    cf_lite::FoundationShims::register_nsuuid_class(&mut rt);
    cf_lite::FoundationShims::register_nslocale_class(&mut rt);
    cf_lite::FoundationShims::register_nstimezone_class(&mut rt);
    cf_lite::FoundationShims::register_nscalendar_class(&mut rt);
    cf_lite::FoundationShims::register_nsnumberformatter_class(&mut rt);
    cf_lite::FoundationShims::register_nsdecimalnumber_class(&mut rt);
    cf_lite::FoundationShims::register_nsindexset_class(&mut rt);
    cf_lite::FoundationShims::register_nsmutableindexset_class(&mut rt);
    cf_lite::FoundationShims::register_nssortdescriptor_class(&mut rt);
    cf_lite::FoundationShims::register_nstimer_class(&mut rt);
    cf_lite::FoundationShims::register_nsthread_class(&mut rt);
    cf_lite::FoundationShims::register_nslock_class(&mut rt);
    cf_lite::FoundationShims::register_nsrecursivelock_class(&mut rt);
    cf_lite::FoundationShims::register_nscondition_class(&mut rt);
    cf_lite::FoundationShims::register_nsconditionlock_class(&mut rt);

    let mut classes: Vec<ClassSummary> = rt
        .classes_by_name_iter()
        .map(|(name, info)| {
            let instance_methods: Vec<String> = info
                .methods
                .iter()
                .map(|m| m.selector.to_string())
                .collect();
            let class_methods: Vec<String> = info
                .class_methods
                .iter()
                .map(|m| m.selector.to_string())
                .collect();
            ClassSummary {
                name: name.to_string(),
                instance_method_count: instance_methods.len(),
                class_method_count: class_methods.len(),
                instance_methods,
                class_methods,
            }
        })
        .collect();
    classes.sort_by(|a, b| a.name.cmp(&b.name));
    classes
}

struct ClassSummary {
    name: String,
    instance_method_count: usize,
    class_method_count: usize,
    instance_methods: Vec<String>,
    class_methods: Vec<String>,
}

const VISIBLE_ROWS: usize = 12;

fn main() {
    let config = WindowConfig {
        title: "Foundation Explorer".into(),
        width: 640.0,
        height: 520.0,
        resizable: true,
    };

    let classes = build_class_data();

    let total_classes = classes.len();
    let total_selectors: usize = classes
        .iter()
        .map(|c| c.instance_method_count + c.class_method_count)
        .sum();

    let mut app = GuiApp::new(config);
    app.set_background(Color::new(0.96, 0.96, 0.98, 1.0));

    // ── Header ───────────────────────────────────────
    let mut title = Label::new(Rect::new(20.0, 10.0, 600.0, 22.0), "Foundation Explorer");
    title.set_text_color(Color::new(0.1, 0.1, 0.4, 1.0));
    app.add_label(title); // label 0

    app.add_label(Label::new(
        Rect::new(20.0, 36.0, 600.0, 18.0),
        format!("{total_classes} classes, {total_selectors} selectors registered"),
    )); // label 1

    // ── Class list (rendered as labels) ──────────────
    // Labels 2..2+VISIBLE_ROWS show class names
    for row in 0..VISIBLE_ROWS {
        let y = 70.0 + row as f64 * 24.0;
        let text = if row < classes.len() {
            format!(
                "{}  ({} inst, {} class)",
                classes[row].name,
                classes[row].instance_method_count,
                classes[row].class_method_count
            )
        } else {
            String::new()
        };
        let mut label = Label::new(Rect::new(20.0, y, 600.0, 20.0), text);
        label.set_text_color(Color::new(0.15, 0.15, 0.15, 1.0));
        app.add_label(label);
    }

    // ── Detail panel ─────────────────────────────────
    // Label for method details (index = 2 + VISIBLE_ROWS)
    let detail_idx = 2 + VISIBLE_ROWS;
    app.add_label(Label::new(
        Rect::new(20.0, 370.0, 600.0, 18.0),
        "Click a class button to see methods.",
    ));

    // Method list labels (detail_idx+1 .. detail_idx+5)
    for i in 0..5 {
        let y = 395.0 + i as f64 * 20.0;
        app.add_label(Label::new(Rect::new(20.0, y, 600.0, 18.0), ""));
    }

    // ── Navigation buttons ───────────────────────────
    // Buttons for each visible class row
    for row in 0..VISIBLE_ROWS {
        let y = 68.0 + row as f64 * 24.0;
        let mut btn = Button::new(Rect::new(540.0, y, 80.0, 22.0), "Show");
        if row >= classes.len() {
            btn.set_enabled(false);
        }
        app.add_button(btn);
    }

    // Scroll buttons
    app.add_button(Button::new(
        Rect::new(20.0, 360.0 - 30.0, 80.0, 24.0),
        "Prev",
    ));
    app.add_button(Button::new(
        Rect::new(110.0, 360.0 - 30.0, 80.0, 24.0),
        "Next",
    ));

    let mut scroll_offset: usize = 0;

    app.on_button_click(move |idx, labels, _tfs| {
        if idx < VISIBLE_ROWS {
            // Show methods for class at scroll_offset + idx
            let class_idx = scroll_offset + idx;
            if class_idx < classes.len() {
                let cls = &classes[class_idx];
                labels[detail_idx].set_text(format!(
                    "{}:  {} instance methods, {} class methods",
                    cls.name, cls.instance_method_count, cls.class_method_count
                ));

                // Show up to 5 method names
                let all_methods: Vec<&str> = cls
                    .instance_methods
                    .iter()
                    .chain(cls.class_methods.iter())
                    .map(|s| s.as_str())
                    .collect();
                for i in 0..5 {
                    if i < all_methods.len() {
                        let prefix = if i < cls.instance_method_count {
                            "-"
                        } else {
                            "+"
                        };
                        labels[detail_idx + 1 + i]
                            .set_text(format!("  {prefix} {}", all_methods[i]));
                    } else {
                        labels[detail_idx + 1 + i].set_text("");
                    }
                }
            }
        } else if idx == VISIBLE_ROWS {
            // Prev
            if scroll_offset >= VISIBLE_ROWS {
                scroll_offset -= VISIBLE_ROWS;
            } else {
                scroll_offset = 0;
            }
            update_class_list(labels, &classes, scroll_offset);
        } else if idx == VISIBLE_ROWS + 1 {
            // Next
            if scroll_offset + VISIBLE_ROWS < classes.len() {
                scroll_offset += VISIBLE_ROWS;
            }
            update_class_list(labels, &classes, scroll_offset);
        }
    });

    app.run();
}

fn update_class_list(labels: &mut [Label], classes: &[ClassSummary], offset: usize) {
    for row in 0..VISIBLE_ROWS {
        let label_idx = 2 + row;
        let class_idx = offset + row;
        if class_idx < classes.len() {
            labels[label_idx].set_text(format!(
                "{}  ({} inst, {} class)",
                classes[class_idx].name,
                classes[class_idx].instance_method_count,
                classes[class_idx].class_method_count
            ));
        } else {
            labels[label_idx].set_text("");
        }
    }
}
