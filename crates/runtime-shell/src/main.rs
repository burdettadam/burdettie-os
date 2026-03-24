//! Interactive runtime demo shell.
//!
//! Provides a REPL-style CLI for running demo fixtures, viewing the
//! compatibility scorecard, and toggling runtime options.

mod commands;
mod scorecard;
mod shell;

fn main() {
    env_logger::init();
    shell::run();
}
