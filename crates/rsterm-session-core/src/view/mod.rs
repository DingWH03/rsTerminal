//! Session-owned terminal view state (scroll, selection, galley cache).

mod galley_cache;
mod selection_state;
mod state;

pub use galley_cache::RowGalleyCache;
pub use selection_state::{CellPos, TerminalSelection, TerminalTouchState, extract_range_text};
pub use state::{PortViewState, TerminalViewState};
