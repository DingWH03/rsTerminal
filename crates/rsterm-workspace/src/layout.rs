//! Reusable workspace layout model and pure tree operations.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct PaneId(pub u64);

impl Default for PaneId {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum SplitAxis {
    Horizontal,
    Vertical,
}

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
            Self::Left | Self::Right => SplitAxis::Horizontal,
            Self::Top | Self::Bottom => SplitAxis::Vertical,
        }
    }

    pub fn new_pane_first(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DropZone {
    Root { edge: DropEdge },
    Pane { pane_id: PaneId, edge: DropEdge },
    PaneCenter { pane_id: PaneId },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SplitNode {
    Leaf {
        pane_id: PaneId,
    },
    Split {
        axis: SplitAxis,
        ratio: f32,
        first: Box<SplitNode>,
        second: Box<SplitNode>,
    },
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PaneState {
    pub session_id: Option<String>,
    /// Index into `Prefs::appearance.pane_accent_colors` (or theme palette).
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

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorkspaceLayout {
    pub root: SplitNode,
    pub panes: HashMap<PaneId, PaneState>,
    pub focused_pane: PaneId,
}

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
            .and_then(|pane| pane.session_id.as_deref())
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
            .any(|pane| pane.session_id.as_deref() == Some(session_id))
    }

    pub fn pane_for_session(&self, session_id: &str) -> Option<PaneId> {
        self.panes
            .iter()
            .find(|(_, pane)| pane.session_id.as_deref() == Some(session_id))
            .map(|(id, _)| *id)
    }

    pub fn swap_panes(&mut self, a: PaneId, b: PaneId) {
        if a == b {
            return;
        }
        let a_session = self.panes.get(&a).and_then(|pane| pane.session_id.clone());
        let b_session = self.panes.get(&b).and_then(|pane| pane.session_id.clone());
        if let Some(pane) = self.panes.get_mut(&a) {
            pane.session_id = b_session;
        }
        if let Some(pane) = self.panes.get_mut(&b) {
            pane.session_id = a_session;
        }
    }

    pub fn insert_session_at_edge(
        &mut self,
        target: PaneId,
        edge: DropEdge,
        session_id: String,
        new_color_index: usize,
    ) -> Option<PaneId> {
        if !self.panes.contains_key(&target) {
            return None;
        }
        if !self.can_split() {
            self.clear_session_except_pane(&session_id, self.focused_pane);
            self.assign_session(self.focused_pane, Some(session_id));
            return Some(self.focused_pane);
        }
        if self.panes.get(&target)?.session_id.as_deref() == Some(session_id.as_str()) {
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
        let session = self
            .panes
            .get(&src)
            .and_then(|pane| pane.session_id.clone())?;
        if let Some(pane) = self.panes.get_mut(&src) {
            pane.session_id = None;
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
            DropZone::Root { edge } => self.insert_session_at_edge(
                self.focused_pane,
                edge,
                session_id.to_string(),
                new_color_index,
            ),
            DropZone::Pane { pane_id, edge } => {
                self.insert_session_at_edge(pane_id, edge, session_id.to_string(), new_color_index)
            }
            DropZone::PaneCenter { pane_id } => {
                self.clear_session_except_pane(session_id, pane_id);
                self.assign_session(pane_id, Some(session_id.to_string()));
                self.focused_pane = pane_id;
                Some(pane_id)
            }
        }
    }

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
            let empty = self
                .panes
                .iter()
                .find(|(_, pane)| pane.session_id.is_none())
                .map(|(id, _)| *id);
            match empty {
                Some(id) if self.close_pane(id) => {}
                _ => break,
            }
        }
    }

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
        let state = match new_session_id {
            Some(session_id) => PaneState::with_session(session_id, new_color_index),
            None => PaneState::with_color(new_color_index),
        };
        self.panes.insert(new_pane, state);
        if !replace_leaf_with_split(&mut self.root, pane_id, axis, 0.5, new_pane) {
            self.panes.remove(&new_pane);
            return None;
        }
        Some(new_pane)
    }

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

    pub fn collapse_to_focused(&mut self) {
        let focused = self.focused_pane;
        let state = self.panes.get(&focused);
        let pane_id = PaneId::new();
        let mut panes = HashMap::new();
        panes.insert(
            pane_id,
            PaneState {
                session_id: state.and_then(|pane| pane.session_id.clone()),
                color_index: state.map(|pane| pane.color_index).unwrap_or(0),
            },
        );
        self.root = SplitNode::Leaf { pane_id };
        self.panes = panes;
        self.focused_pane = pane_id;
    }

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
            let (first_id, second_id) = if new_first {
                (new_pane, *pane_id)
            } else {
                (*pane_id, new_pane)
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
                || replace_leaf_with_split_directed(
                    second, target, axis, ratio, new_pane, new_first,
                )
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
    replace_leaf_with_split_directed(node, target, axis, ratio, new_pane, false)
}

fn remove_pane_from_tree(node: &mut SplitNode, target: PaneId) -> Option<SplitNode> {
    match node {
        SplitNode::Leaf { .. } => None,
        SplitNode::Split { first, second, .. } => {
            if matches!(&**first, SplitNode::Leaf { pane_id } if *pane_id == target) {
                return Some(std::mem::replace(
                    second.as_mut(),
                    SplitNode::Leaf { pane_id: target },
                ));
            }
            if matches!(&**second, SplitNode::Leaf { pane_id } if *pane_id == target) {
                return Some(std::mem::replace(
                    first.as_mut(),
                    SplitNode::Leaf { pane_id: target },
                ));
            }
            if let Some(replacement) = remove_pane_from_tree(first, target) {
                **first = replacement;
            } else if let Some(replacement) = remove_pane_from_tree(second, target) {
                **second = replacement;
            }
            None
        }
    }
}

fn set_ratio_for_pane(node: &mut SplitNode, pane_id: PaneId, ratio: f32) {
    match node {
        SplitNode::Leaf { .. } => {}
        SplitNode::Split {
            ratio: node_ratio,
            first,
            second,
            ..
        } => {
            let in_first = contains_pane(first, pane_id);
            let in_second = contains_pane(second, pane_id);
            if in_first && in_second {
                *node_ratio = ratio;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_and_close_pane() {
        let mut layout = WorkspaceLayout::new_single();
        let root = layout.focused_pane;
        let new = layout
            .split_pane(root, SplitAxis::Horizontal, None, 1)
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
        assert_eq!(
            layout
                .panes
                .get(&new)
                .and_then(|pane| pane.session_id.as_deref()),
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
        assert_eq!(
            layout.insert_session_at_edge(root, DropEdge::Right, "a".into(), 1),
            Some(root)
        );
        assert_eq!(layout.pane_count(), 1);
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
            layout
                .panes
                .get(&a)
                .and_then(|pane| pane.session_id.as_deref()),
            Some("b")
        );
        assert_eq!(
            layout
                .panes
                .get(&b)
                .and_then(|pane| pane.session_id.as_deref()),
            Some("a")
        );
    }
}
