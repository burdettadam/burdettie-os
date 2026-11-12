//! TableView — a data-driven list with rows and columns.
//!
//! Uses a data source pattern: callers provide row count and cell
//! content via the `TableDataSource` trait. The table view handles
//! rendering, selection, and (optionally) scrolling.

use graphics_core::{Color, Font, Point, Rect, Renderer};
use ui_view::View;

/// Provides data to a TableView.
pub trait TableDataSource {
    /// Number of rows in the table.
    fn number_of_rows(&self) -> usize;

    /// Number of columns.
    fn number_of_columns(&self) -> usize {
        1
    }

    /// The text content for a cell at (row, column).
    fn cell_text(&self, row: usize, column: usize) -> String;

    /// Column header title. Default: "Column N".
    fn column_title(&self, column: usize) -> String {
        format!("Column {column}")
    }
}

/// A simple in-memory data source backed by string vectors.
pub struct VecDataSource {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl VecDataSource {
    /// Create with column headers.
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
        }
    }

    /// Add a row of cell values.
    pub fn add_row(&mut self, cells: Vec<String>) {
        self.rows.push(cells);
    }

    /// Clear all rows.
    pub fn clear(&mut self) {
        self.rows.clear();
    }

    /// Number of rows.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }
}

impl TableDataSource for VecDataSource {
    fn number_of_rows(&self) -> usize {
        self.rows.len()
    }

    fn number_of_columns(&self) -> usize {
        self.headers.len()
    }

    fn cell_text(&self, row: usize, column: usize) -> String {
        self.rows
            .get(row)
            .and_then(|r| r.get(column))
            .cloned()
            .unwrap_or_default()
    }

    fn column_title(&self, column: usize) -> String {
        self.headers
            .get(column)
            .cloned()
            .unwrap_or_else(|| format!("Column {column}"))
    }
}

/// A column definition for the table view.
#[derive(Debug, Clone)]
pub struct TableColumn {
    pub title: String,
    pub width: f64,
}

impl TableColumn {
    pub fn new(title: impl Into<String>, width: f64) -> Self {
        Self {
            title: title.into(),
            width,
        }
    }
}

/// A table view that displays rows of data in columns.
pub struct TableView {
    view: View,
    columns: Vec<TableColumn>,
    row_height: f64,
    header_height: f64,
    selected_row: Option<usize>,
    scroll_offset: f64,
    font: Font,
    header_bg: Color,
    header_text_color: Color,
    row_bg: Color,
    alternate_row_bg: Color,
    selection_color: Color,
    text_color: Color,
    shows_header: bool,
}

impl TableView {
    /// Create a table view with frame and column definitions.
    pub fn new(frame: Rect, columns: Vec<TableColumn>) -> Self {
        Self {
            view: View::new(frame),
            columns,
            row_height: 22.0,
            header_height: 24.0,
            selected_row: None,
            scroll_offset: 0.0,
            font: Font::default(),
            header_bg: Color::new(0.9, 0.9, 0.9, 1.0),
            header_text_color: Color::BLACK,
            row_bg: Color::WHITE,
            alternate_row_bg: Color::new(0.96, 0.96, 0.96, 1.0),
            selection_color: Color::new(0.2, 0.5, 0.9, 0.3),
            text_color: Color::BLACK,
            shows_header: true,
        }
    }

    /// The currently selected row, if any.
    pub fn selected_row(&self) -> Option<usize> {
        self.selected_row
    }

    /// Set the selected row.
    pub fn set_selected_row(&mut self, row: Option<usize>) {
        self.selected_row = row;
        self.view.set_needs_display();
    }

    /// Row height in points.
    pub fn row_height(&self) -> f64 {
        self.row_height
    }

    /// Set row height.
    pub fn set_row_height(&mut self, h: f64) {
        self.row_height = h;
        self.view.set_needs_display();
    }

    /// Vertical scroll offset.
    pub fn scroll_offset(&self) -> f64 {
        self.scroll_offset
    }

    /// Set scroll offset (clamped to valid range on draw).
    pub fn set_scroll_offset(&mut self, offset: f64) {
        self.scroll_offset = offset.max(0.0);
        self.view.set_needs_display();
    }

    /// Scroll by a delta.
    pub fn scroll_by(&mut self, dy: f64) {
        self.set_scroll_offset(self.scroll_offset + dy);
    }

    /// Show/hide the column header row.
    pub fn set_shows_header(&mut self, shows: bool) {
        self.shows_header = shows;
    }

    /// Access columns.
    pub fn columns(&self) -> &[TableColumn] {
        &self.columns
    }

    /// Access the underlying view.
    pub fn view(&self) -> &View {
        &self.view
    }

    /// Mutable view access.
    pub fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    /// Given a click Y position (in frame coordinates), return the row index.
    pub fn row_at_point(&self, y: f64, total_rows: usize) -> Option<usize> {
        let header_offset = if self.shows_header {
            self.header_height
        } else {
            0.0
        };
        let local_y = y - self.view.frame.y - header_offset + self.scroll_offset;
        if local_y < 0.0 {
            return None;
        }
        let row = (local_y / self.row_height) as usize;
        if row < total_rows {
            Some(row)
        } else {
            None
        }
    }

    /// Draw the table with data from the given data source.
    pub fn draw(&mut self, renderer: &mut dyn Renderer, data: &dyn TableDataSource) {
        if self.view.hidden {
            return;
        }

        let frame = self.view.frame;

        // Background
        renderer.fill_rect(frame, self.row_bg);

        // Clamp scroll
        let total_rows = data.number_of_rows();
        let content_height = total_rows as f64 * self.row_height;
        let body_height = frame.height
            - if self.shows_header {
                self.header_height
            } else {
                0.0
            };
        let max_scroll = (content_height - body_height).max(0.0);
        self.scroll_offset = self.scroll_offset.clamp(0.0, max_scroll);

        // Header
        let mut body_y = frame.y;
        if self.shows_header {
            let header_rect = Rect::new(frame.x, frame.y, frame.width, self.header_height);
            renderer.fill_rect(header_rect, self.header_bg);

            let mut col_x = frame.x;
            for (ci, col) in self.columns.iter().enumerate() {
                let title = if ci < data.number_of_columns() {
                    data.column_title(ci)
                } else {
                    col.title.clone()
                };
                renderer.draw_text(
                    &title,
                    Point::new(col_x + 4.0, frame.y + 4.0),
                    &self.font,
                    self.header_text_color,
                );
                col_x += col.width;
            }
            // Header bottom border
            renderer.draw_line(
                Point::new(frame.x, frame.y + self.header_height),
                Point::new(frame.x + frame.width, frame.y + self.header_height),
                Color::new(0.7, 0.7, 0.7, 1.0),
                1.0,
            );
            body_y = frame.y + self.header_height;
        }

        // Rows
        let first_visible_row = (self.scroll_offset / self.row_height) as usize;
        let visible_rows = (body_height / self.row_height).ceil() as usize + 1;

        for ri in 0..visible_rows {
            let row = first_visible_row + ri;
            if row >= total_rows {
                break;
            }

            let row_y = body_y + (row as f64 * self.row_height) - self.scroll_offset;
            if row_y + self.row_height < body_y || row_y > frame.y + frame.height {
                continue;
            }

            // Alternating row background
            let bg = if self.selected_row == Some(row) {
                self.selection_color
            } else if row % 2 == 1 {
                self.alternate_row_bg
            } else {
                self.row_bg
            };
            let row_rect = Rect::new(frame.x, row_y, frame.width, self.row_height);
            renderer.fill_rect(row_rect, bg);

            // Cell text
            let mut col_x = frame.x;
            for (ci, col) in self.columns.iter().enumerate() {
                let text = data.cell_text(row, ci);
                renderer.draw_text(
                    &text,
                    Point::new(col_x + 4.0, row_y + 3.0),
                    &self.font,
                    self.text_color,
                );
                col_x += col.width;
            }
        }

        // Border
        renderer.stroke_rect(frame, Color::new(0.7, 0.7, 0.7, 1.0), 1.0);
    }
}

#[cfg(test)]
mod tests;
