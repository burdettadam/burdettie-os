//! ICU-backed internationalization shims for guest Mach-O binaries.
//!
//! Uses [ICU4X](https://github.com/unicode-org/icu4x) — the Unicode
//! Consortium's pure-Rust ICU implementation — to provide real i18n
//! functionality without requiring a system libicu installation.
//!
//! # Apple Stack Mapping
//!
//! ```text
//! Apple:  libicucore.A.dylib  → uregex_*, ucol_*, uloc_*, udat_*, unum_*, ubrk_*
//! Us:     icu-shims           → real ICU4X implementations registered into ShimTable
//! ```
//!
//! # Features
//!
//! - **Collation** (`ucol_*`): Real Unicode-aware string comparison via `icu_collator`
//! - **Locale** (`uloc_*`): Real locale parsing and identification via `icu_locale`
//! - **Segmentation** (`ubrk_*`): Real word/sentence/grapheme break iteration via `icu_segmenter`
//! - **Date formatting** (`udat_*`): Placeholder using `icu_datetime`
//! - **Number formatting** (`unum_*`): Placeholder using `icu_decimal`
//!
//! Symbols that don't have a natural ICU4X equivalent (e.g. `uregex_*`)
//! remain as safe stubs that return appropriate error codes.

// Function pointers are cast to u64 addresses for the emulated Mach-O address space.
#![allow(dead_code, clippy::fn_to_numeric_cast)]

mod collation;
mod calendar;
mod datetime;
mod decimal;
mod locale;
mod segmenter;
mod stubs;

use libsystem_shims::table::ShimTable;

/// Install all ICU-backed shims into the shim table.
///
/// This replaces the stub-only ICU shims from `libsystem-shims` with
/// real implementations where possible:
///
/// - `ucol_*` → real collation via `icu_collator`  
/// - `uloc_*` → real locale via `icu_locale`
/// - `ubrk_*` → real segmentation via `icu_segmenter`
/// - `udat_*` → real date formatting via `icu_datetime`
/// - `unum_*` → real number formatting via `icu_decimal`
/// - `uregex_*` → stubs (ICU4X handles these differently)
pub fn register_icu_shims(table: &mut ShimTable) {
    collation::install(table);
    calendar::install(table);
    locale::install(table);
    segmenter::install(table);
    datetime::install(table);
    decimal::install(table);
    stubs::install_remaining_stubs(table);
    log::debug!("icu-shims: registered ICU4X-backed i18n shims");
}
