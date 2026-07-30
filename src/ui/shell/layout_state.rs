//! Shell layout state — function pane width, workspace split tree, focus.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use crate::ui::function_pane::pages::FunctionPage;

/// Unique pane identifier.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PaneId(pub u64);

impl PaneId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

/// Split direction: horizontal = side-by-side, vertical = stacked.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

/// Edge of a pane or workspace used for drop targeting.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DropEdge {
    Left,
    Right,
    Top,
    Bottom,
}

impl DropEdge {
    pub fn split_axis(self) -> SplitAxis {
        match self {
            DropEdge::Left | DropEdge::Right => SplitAxis::Horizontal,
            DropEdge::Top | DropEdge::Bottom => SplitAxis::Vertical,
        }
    }

    pub fn new_pane_first(self) -> bool {
        matches!(self, DropEdge::Left | DropEdge::Top)
    }
}

/// Drop target during drag-and-drop.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DropZone {
    Root { edge: DropEdge },
    Pane { pane_id: PaneId, edge: DropEdge },
    PaneCenter { pane_id: PaneId },
}

/// Binary split tree node.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SplitNode {
    Leaf { pane_id: PaneId },
    Split {
        axis: SplitAxis,
        ratio: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

/// Per-pane state — references a session by id.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PaneState {
    pub session_id: Option<String>,
    /// Index into `AppSettings::pane_accent_colors` (or theme palette).
    #[serde(default)]
    pub color_index: usize,
}

impl PaneState {
    pub fn with_color(color_index: usize) -> Self {
        Self {
            session_id: None,
            color_index,
        }
    }

    pub fn with_session(session_id: String, color_index: usize) -> Self {
        Self {
            session_id: Some(session_id),
            color_index,
        }
    }
}

/// Workspace multi-pane layout.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub root: SplitNode,
    pub panes: HashMap<PaneId, PaneState>,
    pub focused_pane: PaneId,
}

pub const FUNCTION_MIN_WIDTH: f32 = 200.0;
pub const FUNCTION_MAX_WIDTH: f32 = 360.0;
pub const FUNCTION_DEFAULT_WIDTH: f32 = 220.0;
pub const MIN_PANE_WIDTH: f32 = 120.0;
pub const MIN_PANE_HEIGHT: f32 = 80.0;

impl WorkspaceLayout {
    pub fn new_single() -> Self {
        let pane_id = PaneId::new();
        let mut panes = HashMap::new();
        panes.insert(pane_id, PaneState::with_color(0));
        Self {
            root: SplitNode::Leaf { pane_id },
            panes,
            focused_pane: pane_id,
        }
    }

    pub fn max_panes() -> usize {
        #[cfg(target_os = "android")]
        {
            2
        }
        #[cfg(not(target_os = "android"))]
        {
            4
        }
    }

    pub fn pane_count(&self) -> usize {
        self.panes.len()
    }

    pub fn can_split(&self) -> bool {
        self.pane_count() < Self::max_panes()
    }

    pub fn focused_session_id(&self) -> Option<&str> {
        self.panes
            .get(&self.focused_pane)
            .and_then(|p| p.session_id.as_deref())
    }

    pub fn highlighted_session_id(&self) -> Option<&str> {
        self.focused_session_id()
    }

    pub fn assign_session(&mut self, pane: PaneId, session_id: Option<String>) {
        if let Some(state) = self.panes.get_mut(&pane) {
            state.session_id = session_id;
        }
    }

    pub fn clear_session_everywhere(&mut self, session_id: &str) {
        for state in self.panes.values_mut() {
            if state.session_id.as_deref() == Some(session_id) {
                state.session_id = None;
            }
        }
    }

    /// Keep session only on `except` pane; clear from all others.
    pub fn clear_session_except_pane(&mut self, session_id: &str, except: PaneId) {
        for (id, state) in &mut self.panes {
            if *id != except && state.session_id.as_deref() == Some(session_id) {
                state.session_id = None;
            }
        }
    }

    pub fn is_session_visible(&self, session_id: &str) -> bool {
        self.panes
            .values()
            .any(|p| p.session_id.as_deref() == Some(session_id))
    }

    pub fn pane_for_session(&self, session_id: &str) -> Option<PaneId> {
        self.panes
            .iter()
            .find(|(_, p)| p.session_id.as_deref() == Some(session_id))
            .map(|(id, _)| *id)
    }

    pub fn swap_panes(&mut self, a: PaneId, b: PaneId) {
        if a == b {
            return;
        }
        let sa = self.panes.get(&a).and_then(|p| p.session_id.clone());
        let sb = self.panes.get(&b).and_then(|p| p.session_id.clone());
        if let Some(p) = self.panes.get_mut(&a) {
            p.session_id = sb;
        }
        if let Some(p) = self.panes.get_mut(&b) {
            p.session_id = sa;
        }
    }

    /// Insert session at edge of target pane; returns new pane id.
    pub fn insert_session_at_edge(
        &mut self,
        target: PaneId,
        edge: DropEdge,
        session_id: String,
        new_color_index: usize,
    ) -> Option<PaneId> {
        if self.panes.get(&target).is_none() {
            return None;
        }

        if !self.can_split() {
            self.clear_session_except_pane(&session_id, self.focused_pane);
            self.assign_session(self.focused_pane, Some(session_id));
            return Some(self.focused_pane);
        }

        let already_on_target = self
            .panes
            .get(&target)?
            .session_id
            .as_deref()
            == Some(session_id.as_str());

        if already_on_target {
            self.focused_pane = target;
            return Some(target);
        }

        self.clear_session_except_pane(&session_id, target);

        let new_pane = PaneId::new();
        self.panes.insert(
            new_pane,
            PaneState::with_session(session_id, new_color_index),
        );

        if !replace_leaf_with_split_directed(
            &mut self.root,
            target,
            edge.split_axis(),
            0.5,
            new_pane,
            edge.new_pane_first(),
        ) {
            self.panes.remove(&new_pane);
            return None;
        }

        self.prune_empty_panes();
        Some(new_pane)
    }

    pub fn move_pane_to_edge(
        &mut self,
        src: PaneId,
        target: PaneId,
        edge: DropEdge,
        new_color_index: usize,
    ) -> Option<PaneId> {
        if src == target {
            return None;
        }
        let session = self.panes.get(&src).and_then(|p| p.session_id.clone())?;
        if let Some(p) = self.panes.get_mut(&src) {
            p.session_id = None;
        }
        let new_pane = self.insert_session_at_edge(target, edge, session, new_color_index)?;
        self.prune_empty_panes();
        Some(new_pane)
    }

    pub fn apply_session_drop(
        &mut self,
        session_id: &str,
        zone: DropZone,
        new_color_index: usize,
    ) -> Option<PaneId> {
        match zone {
            DropZone::Root { edge } => {
                let target = self.focused_pane;
                self.insert_session_at_edge(
                    target,
                    edge,
                    session_id.to_string(),
                    new_color_index,
                )
            }
            DropZone::Pane { pane_id, edge } => self.insert_session_at_edge(
                pane_id,
                edge,
                session_id.to_string(),
                new_color_index,
            ),
            DropZone::PaneCenter { pane_id } => {
                self.clear_session_except_pane(session_id, pane_id);
                self.assign_session(pane_id, Some(session_id.to_string()));
                self.focused_pane = pane_id;
                Some(pane_id)
            }
        }
    }

    /// Remove pane from layout without closing the session (minimize).
    pub fn hide_pane(&mut self, pane_id: PaneId) -> bool {
        if self.pane_count() <= 1 {
            return false;
        }
        if self.focused_pane == pane_id {
            self.focused_pane = *self
                .panes
                .keys()
                .find(|id| **id != pane_id)
                .unwrap_or(&pane_id);
        }
        self.close_pane(pane_id)
    }

    fn prune_empty_panes(&mut self) {
        while self.pane_count() > 1 {
            let empty: Vec<PaneId> = self
                .panes
                .iter()
                .filter(|(_, p)| p.session_id.is_none())
                .map(|(id, _)| *id)
                .collect();
            if let Some(id) = empty.first() {
                if !self.close_pane(*id) {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Split `pane_id` into two leaves; returns the new pane id.
    pub fn split_pane(
        &mut self,
        pane_id: PaneId,
        axis: SplitAxis,
        new_session_id: Option<String>,
        new_color_index: usize,
    ) -> Option<PaneId> {
        if !self.can_split() {
            return None;
        }
        let new_pane = PaneId::new();
        self.panes.insert(
            new_pane,
            match new_session_id {
                Some(sid) => PaneState::with_session(sid, new_color_index),
                None => PaneState::with_color(new_color_index),
            },
        );
        if !replace_leaf_with_split(&mut self.root, pane_id, axis, 0.5, new_pane) {
            self.panes.remove(&new_pane);
            return None;
        }
        Some(new_pane)
    }

    /// Remove a pane and merge its sibling into the parent split.
    pub fn close_pane(&mut self, pane_id: PaneId) -> bool {
        if self.pane_count() <= 1 {
            return false;
        }
        if let Some(replacement) = remove_pane_from_tree(&mut self.root, pane_id) {
            self.root = replacement;
        } else if matches!(&self.root, SplitNode::Leaf { pane_id: id } if *id == pane_id) {
            return false;
        }
        self.panes.remove(&pane_id);
        if self.focused_pane == pane_id {
            self.focused_pane = *self.panes.keys().next().unwrap_or(&pane_id);
        }
        true
    }

    pub fn set_split_ratio(&mut self, pane_id: PaneId, ratio: f32) {
        set_ratio_for_pane(&mut self.root, pane_id, ratio.clamp(0.15, 0.85));
    }

    /// Collapse to single pane keeping focused session; other sessions stay in pool.
    pub fn collapse_to_focused(&mut self) {
        let focused = self.focused_pane;
        let session = self
            .panes
            .get(&focused)
            .and_then(|p| p.session_id.clone());
        let pane_id = PaneId::new();
        let color_index = self
            .panes
            .get(&focused)
            .map(|p| p.color_index)
            .unwrap_or(0);
        let mut panes = HashMap::new();
        panes.insert(pane_id, PaneState {
            session_id: session,
            color_index,
        });
        self.root = SplitNode::Leaf { pane_id };
        self.panes = panes;
        self.focused_pane = pane_id;
    }

    /// On narrow screens, collapse to a single pane showing the focused session.
    pub fn collapse_to_single(&mut self) {
        self.collapse_to_focused();
    }
}

fn replace_leaf_with_split_directed(
    node: &mut SplitNode,
    target: PaneId,
    axis: SplitAxis,
    ratio: f32,
    new_pane: PaneId,
    new_first: bool,
) -> bool {
    match node {
        SplitNode::Leaf { pane_id } if *pane_id == target => {
            let old = *pane_id;
            let (first_id, second_id) = if new_first {
                (new_pane, old)
            } else {
                (old, new_pane)
            };
            *node = SplitNode::Split {
                axis,
                ratio,
                first: Box::new(SplitNode::Leaf { pane_id: first_id }),
                second: Box::new(SplitNode::Leaf { pane_id: second_id }),
            };
            true
        }
        SplitNode::Leaf { .. } => false,
        SplitNode::Split { first, second, .. } => {
            replace_leaf_with_split_directed(first, target, axis, ratio, new_pane, new_first)
                || replace_leaf_with_split_directed(second, target, axis, ratio, new_pane, new_first)
        }
    }
}

fn replace_leaf_with_split(
    node: &mut SplitNode,
    target: PaneId,
    axis: SplitAxis,
    ratio: f32,
    new_pane: PaneId,
) -> bool {
    match node {
        SplitNode::Leaf { pane_id } if *pane_id == target => {
            let old = *pane_id;
            *node = SplitNode::Split {
                axis,
                ratio,
                first: Box::new(SplitNode::Leaf { pane_id: old }),
                second: Box::new(SplitNode::Leaf { pane_id: new_pane }),
            };
            true
        }
        SplitNode::Leaf { .. } => false,
        SplitNode::Split { first, second, .. } => {
            replace_leaf_with_split(first, target, axis, ratio, new_pane)
                || replace_leaf_with_split(second, target, axis, ratio, new_pane)
        }
    }
}

fn remove_pane_from_tree(node: &mut SplitNode, target: PaneId) -> Option<SplitNode> {
    match node {
        SplitNode::Leaf { .. } => None,
        SplitNode::Split { first, second, .. } => {
            if matches!(&**first, SplitNode::Leaf { pane_id } if *pane_id == target) {
                let sibling = std::mem::replace(second.as_mut(), SplitNode::Leaf { pane_id: target });
                return Some(sibling);
            }
            if matches!(&**second, SplitNode::Leaf { pane_id } if *pane_id == target) {
                let sibling = std::mem::replace(first.as_mut(), SplitNode::Leaf { pane_id: target });
                return Some(sibling);
            }
            if let Some(replacement) = remove_pane_from_tree(first, target) {
                *first = Box::new(replacement);
            } else if let Some(replacement) = remove_pane_from_tree(second, target) {
                *second = Box::new(replacement);
            }
            None
        }
    }
}

fn set_ratio_for_pane(node: &mut SplitNode, pane_id: PaneId, ratio: f32) {
    match node {
        SplitNode::Leaf { .. } => {}
        SplitNode::Split {
            ratio: r,
            first,
            second,
            ..
        } => {
            let in_first = contains_pane(first, pane_id);
            let in_second = contains_pane(second, pane_id);
            if in_first && in_second {
                *r = ratio;
            } else if in_first {
                set_ratio_for_pane(first, pane_id, ratio);
            } else if in_second {
                set_ratio_for_pane(second, pane_id, ratio);
            }
        }
    }
}

fn contains_pane(node: &SplitNode, pane_id: PaneId) -> bool {
    match node {
        SplitNode::Leaf { pane_id: id } => *id == pane_id,
        SplitNode::Split { first, second, .. } => {
            contains_pane(first, pane_id) || contains_pane(second, pane_id)
        }
    }
}

/// Top-level shell layout state.
#[derive(Clone, Debug)]
pub struct ShellLayout {
    pub function_width: f32,
    pub function_page: FunctionPage,
    pub workspace: WorkspaceLayout,
    /// Settings shown as a centered dialog (not a full-page overlay).
    pub settings_dialog_open: bool,
    /// About / help placeholder dialog.
    pub help_dialog_open: bool,
    /// Saved connections browser (Connection → Open).
    pub connections_dialog_open: bool,
    /// Favorite commands manager (Commands → Manage).
    pub commands_manage_dialog_open: bool,
    /// Auth users manager (Preferences → Users).
    pub users_manage_dialog_open: bool,
}

impl Default for ShellLayout {
    fn default() -> Self {
        Self {
            function_width: FUNCTION_DEFAULT_WIDTH,
            function_page: FunctionPage::Active,
            workspace: WorkspaceLayout::new_single(),
            settings_dialog_open: false,
            help_dialog_open: false,
            connections_dialog_open: false,
            commands_manage_dialog_open: false,
            users_manage_dialog_open: false,
        }
    }
}

impl ShellLayout {
    pub fn from_settings(function_width: Option<f32>) -> Self {
        Self {
            function_width: function_width
                .unwrap_or(FUNCTION_DEFAULT_WIDTH)
                .clamp(FUNCTION_MIN_WIDTH, FUNCTION_MAX_WIDTH),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_close_pane() {
        let mut layout = WorkspaceLayout::new_single();
        let root_pane = layout.focused_pane;
        let new = layout
            .split_pane(root_pane, SplitAxis::Horizontal, None, 1)
            .unwrap();
        assert_eq!(layout.pane_count(), 2);
        assert!(layout.close_pane(new));
        assert_eq!(layout.pane_count(), 1);
    }

    #[test]
    fn insert_session_at_edge_splits() {
        let mut layout = WorkspaceLayout::new_single();
        let root = layout.focused_pane;
        layout.assign_session(root, Some("a".into()));
        let new = layout
            .insert_session_at_edge(root, DropEdge::Right, "b".into(), 1)
            .unwrap();
        assert_eq!(layout.pane_count(), 2);
        assert_eq!(
            layout.panes.get(&new).and_then(|p| p.session_id.as_deref()),
            Some("b")
        );
    }

    #[test]
    fn collapse_to_focused_keeps_session() {
        let mut layout = WorkspaceLayout::new_single();
        let root = layout.focused_pane;
        layout
            .split_pane(root, SplitAxis::Horizontal, Some("other".into()), 1)
            .unwrap();
        layout.assign_session(layout.focused_pane, Some("focus".into()));
        layout.collapse_to_focused();
        assert_eq!(layout.pane_count(), 1);
        assert_eq!(layout.focused_session_id(), Some("focus"));
    }

    #[test]
    fn insert_same_session_at_edge_is_noop() {
        let mut layout = WorkspaceLayout::new_single();
        let root = layout.focused_pane;
        layout.assign_session(root, Some("a".into()));
        let result = layout
            .insert_session_at_edge(root, DropEdge::Right, "a".into(), 1)
            .unwrap();
        assert_eq!(result, root);
        assert_eq!(layout.pane_count(), 1);
        assert_eq!(layout.focused_session_id(), Some("a"));
    }

    #[test]
    fn swap_panes_exchanges_sessions() {
        let mut layout = WorkspaceLayout::new_single();
        let a = layout.focused_pane;
        let b = layout
            .split_pane(a, SplitAxis::Horizontal, Some("b".into()), 1)
            .unwrap();
        layout.assign_session(a, Some("a".into()));
        layout.swap_panes(a, b);
        assert_eq!(
            layout.panes.get(&a).and_then(|p| p.session_id.as_deref()),
            Some("b")
        );
        assert_eq!(
            layout.panes.get(&b).and_then(|p| p.session_id.as_deref()),
            Some("a")
        );
    }
}
