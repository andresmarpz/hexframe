//! Add an action as one line in `actions!` (action.rs), then handle it where it is dispatched.

mod action;
mod combo;
mod keymap;

pub use action::{Action, Group, Scope, Spec};
pub use combo::{KeyCombo, ParseKeyComboError};
pub use keymap::{BindError, Keymap};

#[cfg(test)]
mod tests;
