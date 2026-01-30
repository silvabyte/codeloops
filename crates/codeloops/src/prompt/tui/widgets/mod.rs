//! TUI widgets for the prompt generator.

mod draft;
mod input;
mod question;

// Progress widget removed (Task 3.4: percentage was misleading)
#[allow(dead_code)]
mod progress;

pub use draft::DraftWidget;
pub use input::{ConfirmInput, MultiSelectInput, SelectInput, TextInput};
pub use question::QuestionWidget;
