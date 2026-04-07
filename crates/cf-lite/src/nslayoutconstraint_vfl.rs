//! Visual Format Language (VFL) parser for NSLayoutConstraint.
//!
//! Implements `+[NSLayoutConstraint constraintsWithVisualFormat:options:metrics:views:]`.
//!
//! Supported grammar:
//!   format     = orientation? connection* ;
//!   orientation = 'H:' | 'V:' ;
//!   connection  = superview | view_conn ;
//!   superview   = '|' ;
//!   view_conn   = predicate? '[' view_name predicate_list? ']' predicate? ;
//!   predicate   = '-' | '-' number '-' | '' (flush) ;
//!   view_name   = identifier ;
//!   predicate_list = '(' predicate_item (',' predicate_item)* ')' ;
//!   predicate_item = relation? number priority? | relation? metric_name priority? ;
//!   relation    = '==' | '>=' | '<=' ;
//!
//! NSLayoutAttribute mapping for H: → Left(1)/Right(2), V: → Top(3)/Bottom(4).
//! Default spacing is 8.0.

const DEFAULT_SPACING: f64 = 8.0;

/// Relation constants matching NSLayoutRelation.
const REL_LE: u64 = (-1i64) as u64;
const REL_EQ: u64 = 0;
const REL_GE: u64 = 1;

/// A parsed size predicate (e.g., `(>=50)`, `(==200@750)`, `(==metricName)`).
struct SizePredicate {
    relation: u64,
    constant: f64,
    /// Metric name if the predicate uses a named metric instead of a literal number.
    metric_name: Option<String>,
    priority: f64,
}

/// Element in the VFL parse stream.
enum VflElement {
    Superview,
    View {
        key: String,
        predicates: Vec<SizePredicate>,
    },
    Spacing(f64),
    /// Spacing that uses a named metric (resolved at constraint generation time).
    MetricSpacing(String),
    StandardSpacing,
    Flush,
}

/// Parse a VFL format string into a list of elements.
fn parse_vfl(format: &str) -> (bool, Vec<VflElement>) {
    let mut chars: &str = format;
    let is_vertical;

    // Parse orientation prefix.
    if let Some(rest) = chars.strip_prefix("V:") {
        is_vertical = true;
        chars = rest;
    } else if let Some(rest) = chars.strip_prefix("H:") {
        is_vertical = false;
        chars = rest;
    } else {
        is_vertical = false; // default horizontal
    }

    let mut elements = Vec::new();

    while !chars.is_empty() {
        if let Some(rest) = chars.strip_prefix('|') {
            elements.push(VflElement::Superview);
            chars = rest;
        } else if let Some(rest) = chars.strip_prefix('-') {
            // Could be `-number-`, `-metricName-`, or just `-` (standard spacing).
            if let Some((num, after)) = parse_number_dash(rest) {
                elements.push(VflElement::Spacing(num));
                chars = after;
            } else {
                // Try metric name: `-identifier-`
                let (name, after_name) = parse_identifier(rest);
                if !name.is_empty() {
                    let after_dash = after_name.strip_prefix('-').unwrap_or(after_name);
                    elements.push(VflElement::MetricSpacing(name));
                    chars = after_dash;
                } else {
                    elements.push(VflElement::StandardSpacing);
                    chars = rest;
                }
            }
        } else if let Some(rest) = chars.strip_prefix('[') {
            // Parse view name.
            let (name, after_name) = parse_identifier(rest);
            if name.is_empty() {
                break; // malformed
            }

            // Check for size predicates: `(...)`.
            let (predicates, after_preds) = if let Some(rest2) = after_name.strip_prefix('(') {
                parse_predicates(rest2)
            } else {
                (Vec::new(), after_name)
            };

            // Expect `]`.
            let after_bracket = after_preds.strip_prefix(']').unwrap_or(after_preds);

            elements.push(VflElement::View {
                key: name,
                predicates,
            });
            chars = after_bracket;
        } else {
            // Adjacent views with no separator = flush (0 spacing).
            elements.push(VflElement::Flush);
            // Skip unknown character.
            chars = &chars[1..];
        }
    }

    (is_vertical, elements)
}

/// Try to parse a number followed by `-`. Returns (number, rest_after_dash).
fn parse_number_dash(s: &str) -> Option<(f64, &str)> {
    let mut end = 0;
    let bytes = s.as_bytes();
    while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
        end += 1;
    }
    if end == 0 {
        return None;
    }
    let num: f64 = s[..end].parse().ok()?;
    let rest = &s[end..];
    let rest = rest.strip_prefix('-').unwrap_or(rest);
    Some((num, rest))
}

/// Parse an identifier (alphanumeric + underscore).
fn parse_identifier(s: &str) -> (String, &str) {
    let end = s
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .unwrap_or(s.len());
    (s[..end].to_string(), &s[end..])
}

/// Parse predicate list inside `(...)`. Input starts after `(`.
fn parse_predicates(s: &str) -> (Vec<SizePredicate>, &str) {
    let mut preds = Vec::new();
    let mut chars = s;

    loop {
        // Skip whitespace.
        chars = chars.trim_start();

        // Parse optional relation.
        let (relation, after_rel) = if let Some(rest) = chars.strip_prefix(">=") {
            (REL_GE, rest)
        } else if let Some(rest) = chars.strip_prefix("<=") {
            (REL_LE, rest)
        } else if let Some(rest) = chars.strip_prefix("==") {
            (REL_EQ, rest)
        } else {
            (REL_EQ, chars)
        };

        // Parse number or metric name.
        let mut end = 0;
        let bytes = after_rel.as_bytes();
        while end < bytes.len() && (bytes[end].is_ascii_digit() || bytes[end] == b'.') {
            end += 1;
        }
        let (constant, metric_name) = if end > 0 {
            // Numeric literal.
            (after_rel[..end].parse().unwrap_or(0.0), None)
        } else {
            // Try metric name (identifier).
            let (name, _) = parse_identifier(after_rel);
            if !name.is_empty() {
                end = name.len();
                (0.0, Some(name))
            } else {
                (0.0, None)
            }
        };
        chars = &after_rel[end..];

        // Parse optional priority: @750.
        let priority = if let Some(rest) = chars.strip_prefix('@') {
            let mut pend = 0;
            let pbytes = rest.as_bytes();
            while pend < pbytes.len() && (pbytes[pend].is_ascii_digit() || pbytes[pend] == b'.') {
                pend += 1;
            }
            let p = rest[..pend].parse().unwrap_or(1000.0);
            chars = &rest[pend..];
            p
        } else {
            1000.0
        };

        preds.push(SizePredicate {
            relation,
            constant,
            metric_name,
            priority,
        });

        // Check for comma (more predicates) or closing paren.
        chars = chars.trim_start();
        if let Some(rest) = chars.strip_prefix(',') {
            chars = rest;
        } else if let Some(rest) = chars.strip_prefix(')') {
            chars = rest;
            break;
        } else {
            // Malformed — skip to closing paren or end.
            if let Some(idx) = chars.find(')') {
                chars = &chars[idx + 1..];
            }
            break;
        }
    }

    (preds, chars)
}

// ── Constraint generation ─────────────────────────────────────

fn make_nsstring(s: &str) -> u64 {
    let c = std::ffi::CString::new(s).unwrap_or_default();
    crate::cfstring::shim_cf_string_create_with_cstring(0, c.as_ptr(), 0)
}

fn nsstring_to_rust(ptr: u64) -> String {
    if ptr == 0 {
        return String::new();
    }
    let utf8 = crate::cfstring::shim_nsstring_utf8string(ptr, 0);
    if utf8.is_null() {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(utf8) }
        .to_str()
        .unwrap_or("")
        .to_string()
}

/// Resolve a metric name from the `metrics` NSDictionary (NSNumber values).
fn resolve_metric(metrics: u64, name: &str) -> Option<f64> {
    if metrics == 0 {
        return None;
    }
    let ns_key = make_nsstring(name);
    if ns_key == 0 {
        return None;
    }
    let val = crate::nsdictionary::shim_nsdictionary_object_for_key(metrics, 0, ns_key);
    if val == 0 {
        return None;
    }
    Some(crate::nsnumber::shim_nsnumber_double_value(val, 0))
}

/// Resolve the constant for a size predicate, looking up metric names if present.
fn resolve_metric_value(metrics: u64, pred: &SizePredicate) -> f64 {
    if let Some(ref name) = pred.metric_name {
        resolve_metric(metrics, name).unwrap_or(pred.constant)
    } else {
        pred.constant
    }
}

/// Resolve a view key from the `views` NSDictionary.
fn resolve_view(views: u64, key: &str) -> u64 {
    if views == 0 {
        return 0;
    }
    let ns_key = make_nsstring(key);
    if ns_key == 0 {
        return 0;
    }
    crate::nsdictionary::shim_nsdictionary_object_for_key(views, 0, ns_key)
}

/// Create a constraint between two items/attributes.
fn make_constraint(
    first_item: u64,
    first_attr: u64,
    relation: u64,
    second_item: u64,
    second_attr: u64,
    constant: f64,
    priority: f64,
) -> u64 {
    let c = crate::nslayoutconstraint::shim_nslayoutconstraint_create(
        0, 0, first_item, first_attr, relation, second_item, second_attr, 1.0, constant,
    );
    if c != 0 && (priority - 1000.0).abs() > f64::EPSILON {
        crate::nslayoutconstraint::shim_nslayoutconstraint_set_priority(c, 0, priority);
    }
    c
}

/// `+[NSLayoutConstraint constraintsWithVisualFormat:options:metrics:views:]`
pub extern "C" fn shim_nslayoutconstraint_constraints_with_visual_format(
    _cls: u64,
    _cmd: u64,
    format_str: u64,
    _options: u64,
    metrics: u64,
    views: u64,
) -> u64 {
    if format_str == 0 {
        return crate::nsarray::make_nsarray(&[]);
    }

    let format = nsstring_to_rust(format_str);
    let (is_vertical, elements) = parse_vfl(&format);

    // Choose attributes based on orientation.
    let leading_attr: u64 = if is_vertical { 3 } else { 1 };  // Top / Left
    let trailing_attr: u64 = if is_vertical { 4 } else { 2 }; // Bottom / Right
    let size_attr: u64 = if is_vertical { 8 } else { 7 };     // Height / Width

    let mut constraints: Vec<u64> = Vec::new();

    // Walk elements pairwise: (previous_item, current_item, spacing).
    let mut prev_item: u64 = 0; // 0 = not set, else view pointer
    let mut prev_is_superview = false;
    let mut pending_spacing: Option<f64> = None;

    for elem in &elements {
        match elem {
            VflElement::Superview => {
                if prev_item == 0 && !prev_is_superview {
                    // Leading superview edge.
                    prev_is_superview = true;
                } else {
                    // Trailing superview edge — connect previous view's trailing
                    // to superview's trailing.
                    // Handled when we see the next element or at end.
                    prev_is_superview = true;
                }
            }
            VflElement::StandardSpacing => {
                pending_spacing = Some(DEFAULT_SPACING);
            }
            VflElement::Spacing(s) => {
                pending_spacing = Some(*s);
            }
            VflElement::MetricSpacing(ref name) => {
                pending_spacing = Some(resolve_metric(metrics, name).unwrap_or(DEFAULT_SPACING));
            }
            VflElement::Flush => {
                pending_spacing = Some(0.0);
            }
            VflElement::View { key, predicates } => {
                let view = resolve_view(views, key);
                if view == 0 {
                    continue;
                }

                // Connect to previous.
                let spacing = pending_spacing.unwrap_or(0.0);
                pending_spacing = None;

                if prev_is_superview {
                    // Superview leading → view leading.
                    // view.leading = superview.leading + spacing
                    // We need the superview of this view.
                    let superview = crate::nsview::shim_nsview_superview(view, 0);
                    if superview != 0 {
                        let c = make_constraint(
                            view, leading_attr, REL_EQ, superview, leading_attr, spacing, 1000.0,
                        );
                        if c != 0 {
                            constraints.push(c);
                        }
                    }
                    prev_is_superview = false;
                } else if prev_item != 0 {
                    // prev.trailing → view.leading with spacing.
                    // view.leading = prev.trailing + spacing
                    let c = make_constraint(
                        view, leading_attr, REL_EQ, prev_item, trailing_attr, spacing, 1000.0,
                    );
                    if c != 0 {
                        constraints.push(c);
                    }
                }

                // Emit size predicates.
                for pred in predicates {
                    let constant = resolve_metric_value(metrics, pred);
                    let c = make_constraint(
                        view, size_attr, pred.relation, 0, 0, constant, pred.priority,
                    );
                    if c != 0 {
                        constraints.push(c);
                    }
                }

                prev_item = view;
            }
        }
    }

    // Handle trailing superview.
    if prev_is_superview && prev_item != 0 {
        let spacing = pending_spacing.unwrap_or(0.0);
        let superview = crate::nsview::shim_nsview_superview(prev_item, 0);
        if superview != 0 {
            // superview.trailing = prev.trailing + spacing
            let c = make_constraint(
                superview, trailing_attr, REL_EQ, prev_item, trailing_attr, spacing, 1000.0,
            );
            if c != 0 {
                constraints.push(c);
            }
        }
    }

    crate::nsarray::make_nsarray(&constraints)
}

// ── Tests ──────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shim_test_helpers::*;

    fn make_view_with_superview() -> (u64, u64) {
        let parent = crate::nsview::alloc_view_with_frame(0.0, 0.0, 400.0, 300.0);
        let child = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        crate::nsview::shim_nsview_add_subview(parent, 0, child);
        (parent, child)
    }

    // ── parse_vfl ──────────────────────────────────────────────

    #[test]
    fn parse_horizontal_default() {
        let (is_vert, elems) = parse_vfl("H:[view1]-[view2]");
        assert!(!is_vert);
        assert!(elems.len() >= 3);
    }

    #[test]
    fn parse_vertical() {
        let (is_vert, _) = parse_vfl("V:[view1]-[view2]");
        assert!(is_vert);
    }

    #[test]
    fn parse_superview_edges() {
        let (_, elems) = parse_vfl("H:|-[view1]-|");
        // Should have: Superview, StdSpacing, View, StdSpacing, Superview
        let sv_count = elems.iter().filter(|e| matches!(e, VflElement::Superview)).count();
        assert_eq!(sv_count, 2);
    }

    #[test]
    fn parse_size_predicate() {
        let (_, elems) = parse_vfl("H:[view1(>=50)]");
        let view = elems.iter().find(|e| matches!(e, VflElement::View { .. }));
        assert!(view.is_some());
        if let Some(VflElement::View { predicates, .. }) = view {
            assert_eq!(predicates.len(), 1);
            assert_eq!(predicates[0].relation, REL_GE);
            assert!((predicates[0].constant - 50.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn parse_explicit_spacing() {
        let (_, elems) = parse_vfl("H:[v1]-20-[v2]");
        let spacing = elems.iter().find(|e| matches!(e, VflElement::Spacing(_)));
        assert!(spacing.is_some());
        if let Some(VflElement::Spacing(s)) = spacing {
            assert!((*s - 20.0).abs() < f64::EPSILON);
        }
    }

    #[test]
    fn parse_size_with_priority() {
        let (_, elems) = parse_vfl("V:[view1(==200@750)]");
        if let Some(VflElement::View { predicates, .. }) =
            elems.iter().find(|e| matches!(e, VflElement::View { .. }))
        {
            assert_eq!(predicates.len(), 1);
            assert!((predicates[0].constant - 200.0).abs() < f64::EPSILON);
            assert!((predicates[0].priority - 750.0).abs() < f64::EPSILON);
        }
    }

    // ── constraint generation ──────────────────────────────────

    #[test]
    fn constraints_from_simple_format() {
        let (parent, child) = make_view_with_superview();

        let views_keys = [make_nsstring("child")];
        let views_vals = [child];
        let views = crate::nsdictionary::make_nsdictionary(&views_keys, &views_vals);

        let format = make_nsstring("H:|-[child]-|");
        let result = shim_nslayoutconstraint_constraints_with_visual_format(
            0, 0, format, 0, 0, views,
        );
        let count = crate::nsarray::shim_nsarray_count(result, 0);
        // Should produce 2 constraints: superview.leading→child.leading, superview.trailing→child.trailing
        assert_eq!(count, 2, "expected 2 constraints from '|-[child]-|', got {count}");

        free_obj(parent);
    }

    #[test]
    fn constraints_with_size_predicate() {
        let (parent, child) = make_view_with_superview();

        let views_keys = [make_nsstring("child")];
        let views_vals = [child];
        let views = crate::nsdictionary::make_nsdictionary(&views_keys, &views_vals);

        let format = make_nsstring("H:[child(>=100)]");
        let result = shim_nslayoutconstraint_constraints_with_visual_format(
            0, 0, format, 0, 0, views,
        );
        let count = crate::nsarray::shim_nsarray_count(result, 0);
        // 1 size constraint: width >= 100
        assert_eq!(count, 1, "expected 1 size constraint, got {count}");

        let c = crate::nsarray::shim_nsarray_object_at_index(result, 0, 0);
        let first_attr = unsafe { *((c as usize + 16) as *const u64) };
        let relation = unsafe { *((c as usize + 24) as *const u64) };
        let constant = unsafe { *((c as usize + 56) as *const f64) };
        assert_eq!(first_attr, 7); // Width
        assert_eq!(relation, REL_GE);
        assert!((constant - 100.0).abs() < f64::EPSILON);

        free_obj(parent);
    }

    #[test]
    fn constraints_two_views_with_spacing() {
        let parent = crate::nsview::alloc_view_with_frame(0.0, 0.0, 400.0, 300.0);
        let v1 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        let v2 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        crate::nsview::shim_nsview_add_subview(parent, 0, v1);
        crate::nsview::shim_nsview_add_subview(parent, 0, v2);

        let keys = [
            make_nsstring("v1"),
            make_nsstring("v2"),
        ];
        let vals = [v1, v2];
        let views = crate::nsdictionary::make_nsdictionary(&keys, &vals);

        let format = make_nsstring("H:[v1]-[v2]");
        let result = shim_nslayoutconstraint_constraints_with_visual_format(
            0, 0, format, 0, 0, views,
        );
        let count = crate::nsarray::shim_nsarray_count(result, 0);
        // 1 constraint: v2.leading = v1.trailing + 8
        assert_eq!(count, 1, "expected 1 spacing constraint, got {count}");

        free_obj(parent);
    }

    #[test]
    fn vertical_format() {
        let parent = crate::nsview::alloc_view_with_frame(0.0, 0.0, 400.0, 300.0);
        let child = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        crate::nsview::shim_nsview_add_subview(parent, 0, child);

        let keys = [make_nsstring("child")];
        let vals = [child];
        let views = crate::nsdictionary::make_nsdictionary(&keys, &vals);

        let format = make_nsstring("V:[child(==100)]");
        let result = shim_nslayoutconstraint_constraints_with_visual_format(
            0, 0, format, 0, 0, views,
        );
        let count = crate::nsarray::shim_nsarray_count(result, 0);
        assert_eq!(count, 1);

        let c = crate::nsarray::shim_nsarray_object_at_index(result, 0, 0);
        let first_attr = unsafe { *((c as usize + 16) as *const u64) };
        assert_eq!(first_attr, 8); // Height
        free_obj(parent);
    }

    #[test]
    fn nil_format_returns_empty_array() {
        let result = shim_nslayoutconstraint_constraints_with_visual_format(0, 0, 0, 0, 0, 0);
        let count = crate::nsarray::shim_nsarray_count(result, 0);
        assert_eq!(count, 0);
    }

    // ── metric name resolution ─────────────────────────────────

    #[test]
    fn metric_in_size_predicate() {
        let parent = crate::nsview::alloc_view_with_frame(0.0, 0.0, 400.0, 300.0);
        let child = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        crate::nsview::shim_nsview_add_subview(parent, 0, child);

        let views_keys = [make_nsstring("child")];
        let views_vals = [child];
        let views = crate::nsdictionary::make_nsdictionary(&views_keys, &views_vals);

        // Metrics: { "myWidth": 150 }
        let metric_keys = [make_nsstring("myWidth")];
        let metric_vals = [crate::nsnumber::make_nsnumber_double(150.0)];
        let metrics = crate::nsdictionary::make_nsdictionary(&metric_keys, &metric_vals);

        let format = make_nsstring("H:[child(==myWidth)]");
        let result = shim_nslayoutconstraint_constraints_with_visual_format(
            0, 0, format, 0, metrics, views,
        );
        let count = crate::nsarray::shim_nsarray_count(result, 0);
        assert_eq!(count, 1, "expected 1 constraint, got {count}");

        let c = crate::nsarray::shim_nsarray_object_at_index(result, 0, 0);
        let constant = unsafe { *((c as usize + 56) as *const f64) };
        assert!((constant - 150.0).abs() < f64::EPSILON, "expected metric value 150, got {constant}");

        free_obj(parent);
    }

    #[test]
    fn metric_in_spacing() {
        let parent = crate::nsview::alloc_view_with_frame(0.0, 0.0, 400.0, 300.0);
        let v1 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        let v2 = crate::nsview::alloc_view_with_frame(0.0, 0.0, 100.0, 50.0);
        crate::nsview::shim_nsview_add_subview(parent, 0, v1);
        crate::nsview::shim_nsview_add_subview(parent, 0, v2);

        let keys = [make_nsstring("v1"), make_nsstring("v2")];
        let vals = [v1, v2];
        let views = crate::nsdictionary::make_nsdictionary(&keys, &vals);

        let metric_keys = [make_nsstring("gap")];
        let metric_vals = [crate::nsnumber::make_nsnumber_double(30.0)];
        let metrics = crate::nsdictionary::make_nsdictionary(&metric_keys, &metric_vals);

        let format = make_nsstring("H:[v1]-gap-[v2]");
        let result = shim_nslayoutconstraint_constraints_with_visual_format(
            0, 0, format, 0, metrics, views,
        );
        let count = crate::nsarray::shim_nsarray_count(result, 0);
        assert_eq!(count, 1, "expected 1 spacing constraint, got {count}");

        let c = crate::nsarray::shim_nsarray_object_at_index(result, 0, 0);
        let constant = unsafe { *((c as usize + 56) as *const f64) };
        assert!((constant - 30.0).abs() < f64::EPSILON, "expected metric spacing 30, got {constant}");

        free_obj(parent);
    }

    #[test]
    fn parse_metric_name_in_predicate() {
        let (_, elems) = parse_vfl("H:[view1(==myMetric)]");
        if let Some(VflElement::View { predicates, .. }) =
            elems.iter().find(|e| matches!(e, VflElement::View { .. }))
        {
            assert_eq!(predicates.len(), 1);
            assert_eq!(predicates[0].metric_name.as_deref(), Some("myMetric"));
        } else {
            panic!("expected View element");
        }
    }

    #[test]
    fn parse_metric_name_in_spacing() {
        let (_, elems) = parse_vfl("H:[v1]-gap-[v2]");
        let spacing = elems.iter().find(|e| matches!(e, VflElement::MetricSpacing(_)));
        assert!(spacing.is_some(), "expected MetricSpacing element");
        if let Some(VflElement::MetricSpacing(name)) = spacing {
            assert_eq!(name, "gap");
        }
    }
}
