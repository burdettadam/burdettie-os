//! Concrete UI controls: Label, Button, TextField, ScrollView, TableView, SplitView,
//! Checkbox, RadioButton, Slider, ProgressBar, ComboBox, TabView, ImageView.
//!
//! Each control wraps a `View` and adds specialized behavior.
//!
//! Also provides [`AppScene`] — the backend-agnostic rendering and interaction
//! kernel shared by both `GuiApp` (native windowing) and `HeadlessApp` (testing),
//! and layout helpers (StackView, layout_vertical, layout_horizontal).

mod button;
mod checkbox;
mod combobox;
mod headless;
mod imageview;
mod label;
pub mod layout;
mod progressbar;
mod radiobutton;
mod scene;
mod scrollview;
mod slider;
mod splitview;
pub mod tableview;
mod tabview;
mod textfield;

pub use button::Button;
pub use checkbox::Checkbox;
pub use combobox::ComboBox;
pub use headless::HeadlessApp;
pub use imageview::{ContentMode, ImageView};
pub use label::Label;
pub use layout::{Axis, Distribution, StackView};
pub use progressbar::ProgressBar;
pub use radiobutton::{select_in_group, RadioButton};
pub use scene::AppScene;
pub use scrollview::ScrollView;
pub use slider::Slider;
pub use splitview::{SplitOrientation, SplitView};
pub use tabview::TabView;
pub use tableview::{TableColumn, TableDataSource, TableView, VecDataSource};
pub use textfield::TextField;
