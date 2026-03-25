use super::*;

fn sample_data() -> VecDataSource {
    let mut ds = VecDataSource::new(vec!["Name".into(), "Value".into()]);
    ds.add_row(vec!["alpha".into(), "1".into()]);
    ds.add_row(vec!["beta".into(), "2".into()]);
    ds.add_row(vec!["gamma".into(), "3".into()]);
    ds
}

#[test]
fn vec_data_source() {
    let ds = sample_data();
    assert_eq!(ds.number_of_rows(), 3);
    assert_eq!(ds.number_of_columns(), 2);
    assert_eq!(ds.cell_text(0, 0), "alpha");
    assert_eq!(ds.cell_text(1, 1), "2");
    assert_eq!(ds.column_title(0), "Name");
}

#[test]
fn vec_data_source_out_of_bounds() {
    let ds = sample_data();
    assert_eq!(ds.cell_text(99, 0), "");
    assert_eq!(ds.cell_text(0, 99), "");
}

#[test]
fn table_view_basic() {
    let tv = TableView::new(
        Rect::new(0.0, 0.0, 300.0, 200.0),
        vec![TableColumn::new("A", 150.0), TableColumn::new("B", 150.0)],
    );
    assert_eq!(tv.columns().len(), 2);
    assert!(tv.selected_row().is_none());
}

#[test]
fn table_view_selection() {
    let mut tv = TableView::new(
        Rect::new(0.0, 0.0, 300.0, 200.0),
        vec![TableColumn::new("Name", 300.0)],
    );
    tv.set_selected_row(Some(1));
    assert_eq!(tv.selected_row(), Some(1));
    tv.set_selected_row(None);
    assert!(tv.selected_row().is_none());
}

#[test]
fn table_view_row_at_point() {
    let tv = TableView::new(
        Rect::new(0.0, 0.0, 300.0, 200.0),
        vec![TableColumn::new("X", 300.0)],
    );
    // Header is 24, row height 22
    // Row 0 starts at y=24, row 1 at y=46
    let row = tv.row_at_point(30.0, 5);
    assert_eq!(row, Some(0));
    let row = tv.row_at_point(50.0, 5);
    assert_eq!(row, Some(1));
    // Before header
    let row = tv.row_at_point(10.0, 5);
    assert!(row.is_none());
}

#[test]
fn table_view_draw() {
    let mut tv = TableView::new(
        Rect::new(0.0, 0.0, 300.0, 200.0),
        vec![TableColumn::new("Name", 150.0), TableColumn::new("Val", 150.0)],
    );
    let ds = sample_data();
    tv.set_selected_row(Some(1));
    let mut surface = graphics_core::Surface::new(300, 200);
    let mut renderer = graphics_core::SoftwareRenderer::new(&mut surface);
    tv.draw(&mut renderer, &ds); // should not panic
}

#[test]
fn table_view_scroll() {
    let mut tv = TableView::new(
        Rect::new(0.0, 0.0, 300.0, 100.0),
        vec![TableColumn::new("X", 300.0)],
    );
    tv.scroll_by(20.0);
    assert_eq!(tv.scroll_offset(), 20.0);
    tv.set_scroll_offset(-10.0);
    assert_eq!(tv.scroll_offset(), 0.0); // clamped
}

#[test]
fn vec_data_source_clear() {
    let mut ds = sample_data();
    assert_eq!(ds.row_count(), 3);
    ds.clear();
    assert_eq!(ds.row_count(), 0);
}
