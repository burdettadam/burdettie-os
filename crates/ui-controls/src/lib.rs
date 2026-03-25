//! Concrete UI controls: Label, Button, TextField, ScrollView, TableView, SplitView.
//!
//! Each control wraps a `View` and adds specialized behavior.
//!
//! Also provides [`AppScene`] — the backend-agnostic rendering and interaction
//! kernel shared by both `GuiApp` (native windowing) and `HeadlessApp` (testing),
//! and layout helpers (StackView, layout_vertical, layout_horizontal).

mod button;
mod headless;
mod label;
pub mod layout;
mod scene;
mod scrollview;
mod splitview;
pub mod tableview;
mod textfield;

pub use button::Button;
pub use headless::HeadlessApp;
pub use label::Label;
pub use layout::{Axis, Distribution, StackView};
pub use scene::AppScene;
pub use scrollview::ScrollView;
pub use splitview::{SplitOrientation, SplitView};
pub use tableview::{TableColumn, TableDataSource, TableView, VecDataSource};
pub use textfield::TextField;
