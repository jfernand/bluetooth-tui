//! A minimal, backend-agnostic key event - just the variants `App`
//! actually matches on. Deliberately not `crossterm::event::KeyCode`:
//! this crate is shared with a wasm32 frontend, and crossterm's
//! terminal-I/O layer (raw mode, window size, ...) doesn't compile
//! there at all, even just to get at the event-type definitions. Each
//! frontend translates its own key event type into this one.
/// A single backend-agnostic key press.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    /// Up arrow.
    Up,
    /// Down arrow.
    Down,
    /// Left arrow.
    Left,
    /// Right arrow.
    Right,
    /// Enter/Return.
    Enter,
    /// Escape.
    Esc,
    /// Tab.
    Tab,
    /// Backspace.
    Backspace,
    /// Any plain character key.
    Char(char),
}
