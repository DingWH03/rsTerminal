//! Terminal-specific UI state, measurement, and painting.

pub(crate) mod cursor;
pub(crate) mod galley_cache;
pub(crate) mod metrics;
pub(crate) mod selection_state;
pub(crate) mod state;

pub use galley_cache::RowGalleyCache;
pub use selection_state::{CellPos, TerminalSelection, TerminalTouchState, extract_range_text};
pub use state::{PortViewState, TerminalViewState};
