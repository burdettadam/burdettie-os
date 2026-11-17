//! Regression tests use the in-memory backend; no user clipboard is accessed.
use super::*;

#[test]
fn stale_writers_allocate_from_shared_generation() {
    let (_guard, mut first) = tests::fresh_pasteboard();
    let mut stale = NSPasteboard::general();
    first.set_string("first", NS_PASTEBOARD_TYPE_STRING);
    first.set_string("second", NS_PASTEBOARD_TYPE_STRING);
    assert_eq!(first.change_count(), 2);
    assert_eq!(stale.change_count(), 0);
    stale.set_string("third", NS_PASTEBOARD_TYPE_STRING);
    assert_eq!(stale.change_count(), 3);
    let current = NSPasteboard::general();
    assert_eq!(current.change_count(), 3);
    assert_eq!(
        current.string_for_type(NS_PASTEBOARD_TYPE_STRING),
        Some("third")
    );
}

#[test]
fn stale_clear_cannot_rewind_shared_generation() {
    let (_guard, mut first) = tests::fresh_pasteboard();
    let mut stale = NSPasteboard::general();
    for _ in 0..8 {
        first.set_string("value", NS_PASTEBOARD_TYPE_STRING);
    }
    assert_eq!(stale.clear_contents(), 9);
    let current = NSPasteboard::general();
    assert_eq!(current.change_count(), 9);
    assert!(current.types().is_empty());
}

#[test]
fn snapshots_remain_stable_until_explicit_refresh() {
    let (_guard, mut first) = tests::fresh_pasteboard();
    let mut snapshot = NSPasteboard::general();
    first.set_string("new", NS_PASTEBOARD_TYPE_STRING);
    assert_eq!(snapshot.change_count(), 0);
    assert!(snapshot.types().is_empty());
    snapshot.refresh();
    assert_eq!(snapshot.change_count(), 1);
    assert_eq!(
        snapshot.string_for_type(NS_PASTEBOARD_TYPE_STRING),
        Some("new")
    );
}

#[test]
fn parallel_writers_publish_256_distinct_generations() {
    let (_guard, _) = tests::fresh_pasteboard();
    let start = std::sync::Barrier::new(8);
    let mut generations = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|worker| {
                let start = &start;
                scope.spawn(move || {
                    let mut pb = NSPasteboard::general();
                    start.wait();
                    let mut generations = Vec::new();
                    for sequence in 0..32 {
                        pb.set_string(&format!("{worker}:{sequence}"), NS_PASTEBOARD_TYPE_STRING);
                        generations.push(pb.change_count());
                    }
                    generations
                })
            })
            .collect();
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("clipboard writer panicked"))
            .collect::<Vec<_>>()
    });
    generations.sort_unstable();
    assert_eq!(generations, (1..=256).collect::<Vec<u64>>());
    let current = NSPasteboard::general();
    assert_eq!(current.change_count(), 256);
    let text = current.string_for_type(NS_PASTEBOARD_TYPE_STRING).unwrap();
    assert!(text.ends_with(":31"));
}

#[test]
fn explicit_equal_writes_still_advance_generation() {
    let (_guard, mut pb) = tests::fresh_pasteboard();
    pb.set_string("equal", NS_PASTEBOARD_TYPE_STRING);
    pb.set_string("equal", NS_PASTEBOARD_TYPE_STRING);
    assert_eq!(pb.change_count(), 2);
}

#[test]
fn counter_saturates_without_rewinding_or_losing_contents() {
    let (_guard, mut stale) = tests::fresh_pasteboard();
    shared::store_snapshot(&HashMap::new(), u64::MAX - 1);
    stale.set_string("last", NS_PASTEBOARD_TYPE_STRING);
    assert_eq!(stale.change_count(), u64::MAX);
    stale.set_string("still published", NS_PASTEBOARD_TYPE_STRING);
    assert_eq!(stale.change_count(), u64::MAX);
    let current = NSPasteboard::general();
    assert_eq!(current.change_count(), u64::MAX);
    assert_eq!(
        current.string_for_type(NS_PASTEBOARD_TYPE_STRING),
        Some("still published")
    );
}

#[test]
fn observed_host_text_is_preserved_in_shared_snapshot() {
    let (_guard, _) = tests::fresh_pasteboard();
    let mut observed = HashMap::new();
    assert_eq!(shared::observe_plain_text(&mut observed, "host".into()), 1);
    let current = NSPasteboard::general();
    assert_eq!(current.change_count(), 1);
    assert_eq!(
        current.string_for_type(NS_PASTEBOARD_TYPE_STRING),
        Some("host")
    );
    assert_eq!(current.contents, observed);
}

#[test]
fn empty_host_text_replaces_stale_text_and_binary_formats() {
    let (_guard, mut pb) = tests::fresh_pasteboard();
    pb.set_string("old", NS_PASTEBOARD_TYPE_STRING);
    pb.set_data(b"image", NS_PASTEBOARD_TYPE_PNG);
    let mut observed = HashMap::new();
    assert_eq!(shared::observe_plain_text(&mut observed, String::new()), 3);
    assert_eq!(observed.len(), 1);
    assert_eq!(observed.get(NS_PASTEBOARD_TYPE_STRING), Some(&Vec::new()));
    let current = NSPasteboard::general();
    assert_eq!(current.string_for_type(NS_PASTEBOARD_TYPE_STRING), Some(""));
    assert!(current.data_for_type(NS_PASTEBOARD_TYPE_PNG).is_none());
}

#[test]
fn identical_host_text_does_not_advance_or_drop_known_formats() {
    let (_guard, mut pb) = tests::fresh_pasteboard();
    pb.set_string("same", NS_PASTEBOARD_TYPE_STRING);
    pb.set_data(b"pdf", NS_PASTEBOARD_TYPE_PDF);
    let mut observed = HashMap::new();
    for _ in 0..5 {
        assert_eq!(shared::observe_plain_text(&mut observed, "same".into()), 2);
    }
    assert_eq!(observed.get(NS_PASTEBOARD_TYPE_PDF), Some(&b"pdf".to_vec()));
    assert_eq!(observed, pb.contents);
}

#[test]
fn changed_host_text_invalidates_unobserved_old_formats() {
    let (_guard, mut pb) = tests::fresh_pasteboard();
    pb.set_string("old", NS_PASTEBOARD_TYPE_STRING);
    pb.set_data(b"pdf", NS_PASTEBOARD_TYPE_PDF);
    pb.set_data(b"png", NS_PASTEBOARD_TYPE_PNG);
    let mut observed = HashMap::new();
    assert_eq!(shared::observe_plain_text(&mut observed, "new".into()), 4);
    assert_eq!(observed.len(), 1);
    assert_eq!(observed.get(NS_PASTEBOARD_TYPE_STRING), Some(&b"new".to_vec()));
}

#[test]
fn stale_write_follows_an_observed_host_generation() {
    let (_guard, mut stale) = tests::fresh_pasteboard();
    let mut observed = HashMap::new();
    shared::observe_plain_text(&mut observed, "host one".into());
    shared::observe_plain_text(&mut observed, "host two".into());
    stale.set_string("local", NS_PASTEBOARD_TYPE_STRING);
    assert_eq!(stale.change_count(), 3);
    let current = NSPasteboard::general();
    assert_eq!(current.contents, stale.contents);
    assert_eq!(current.change_count(), 3);
}

#[test]
fn valid_host_text_replaces_invalid_utf8_snapshot() {
    let (_guard, mut pb) = tests::fresh_pasteboard();
    pb.set_data(&[0xff], NS_PASTEBOARD_TYPE_STRING);
    assert!(pb.string_for_type(NS_PASTEBOARD_TYPE_STRING).is_none());
    let mut observed = HashMap::new();
    assert_eq!(shared::observe_plain_text(&mut observed, "valid".into()), 2);
    assert_eq!(
        observed.get(NS_PASTEBOARD_TYPE_STRING),
        Some(&b"valid".to_vec())
    );
}

#[test]
fn empty_text_and_absent_text_are_distinct_snapshots() {
    let (_guard, _) = tests::fresh_pasteboard();
    let mut observed = HashMap::new();
    assert_eq!(shared::observe_plain_text(&mut observed, String::new()), 1);
    assert_eq!(shared::observe_plain_text(&mut observed, String::new()), 1);
    assert!(observed.contains_key(NS_PASTEBOARD_TYPE_STRING));
}
