#[path = "darwin_stubs/core_text.rs"]
pub mod core_text;

use crate::table::ShimTable;

pub fn install_darwin_stubs(table: &mut ShimTable) {
    core_text::CoreTextStubs::install(table);
    log::debug!("darwin_stubs: registered non-Unix portable stubs");
}
