//! Workspace layout, split chrome, and pluggable pane content.

pub mod content;
pub mod layout;
pub mod split_handle;

pub use content::{ContentAction, ContentTickCtx, ContentUiCtx, WorkspaceContent, WorkspaceHost};
pub use layout::{
    DropEdge, DropZone, MIN_PANE_HEIGHT, MIN_PANE_WIDTH, PaneId, PaneState, SplitAxis, SplitNode,
    WorkspaceLayout,
};
