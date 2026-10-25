//! Terminal-agnostic application state, edit history, and Cmd commands.

pub mod app;
pub mod buffer;
pub mod cmd;
pub mod focus;
pub mod guide;
pub mod help;
pub mod history;
pub mod input;
pub mod keys;
pub mod manual;
pub mod output;
pub mod tutorial;

pub use app::{App, Page, Pending};
pub use focus::{Focus, View};
pub use output::{OutKind, OutLine};
