//! `WorkspaceContent` adapter for file-manager sessions.

use std::any::Any;

use rsterm_data::prefs::{
    FileManagerPrefs, FileManagerUiState, PrefsFilePaneLayout, PrefsFileViewMode, load_prefs,
    save_prefs,
};
use rsterm_session_core::FileManagerSession;
use rsterm_uiframe::PaneChrome;
use rsterm_uiframe::file_list::{FileDetailsColumns, FilePaneLayout, FileViewMode};
use rsterm_workspace::{ContentAction, ContentUiCtx, WorkspaceContent};

use crate::page::file_manager_view;

/// Which FM pane owns Details column widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailsPaneSide {
    Left,
    Right,
}

/// Orphan-rule newtype owning a [`FileManagerSession`].
pub struct FileManagerContent {
    pub inner: FileManagerSession,
    /// Runtime layout (persisted via prefs when changed).
    pub pane_layout: FilePaneLayout,
    /// Runtime list presentation mode (persisted via prefs when changed).
    pub view_mode: FileViewMode,
    /// Left / remote Details column widths.
    pub details_columns_left: Option<FileDetailsColumns>,
    /// Right Details column widths (independent of left).
    pub details_columns_right: Option<FileDetailsColumns>,
    /// Left pane fraction of dual layout (`0.15..=0.85`).
    pub dual_split: f32,
    /// Whether the advanced search strip under the top bar is open.
    pub search_panel_open: bool,
    /// FM settings gear popup open.
    pub settings_menu: rsterm_uiframe::PopupMenuState,
    /// Unified hover / detail panel state.
    pub hover_panel: rsterm_uiframe::hover_panel::HoverPanelState,
    /// Touch multiselect overlay state.
    pub touch_multiselect: crate::page::touch_multiselect::TouchMultiselectState,
    pub touch_ops_menu: rsterm_uiframe::PopupMenuState,
    pub pending_open_settings: bool,
    /// Pending prefs write for the host app to merge into in-memory `Prefs`.
    pub pending_prefs: Option<FileManagerPrefs>,
    /// Pending ui_state write for the host app to merge into in-memory `Prefs`.
    pub pending_ui_state: Option<FileManagerUiState>,
}

pub fn prefs_to_view_mode(m: PrefsFileViewMode) -> FileViewMode {
    match m {
        PrefsFileViewMode::List => FileViewMode::List,
        PrefsFileViewMode::Details => FileViewMode::Details,
        PrefsFileViewMode::IconsSmall => FileViewMode::IconsSmall,
        PrefsFileViewMode::IconsLarge => FileViewMode::IconsLarge,
    }
}

pub fn prefs_to_pane_layout(l: PrefsFilePaneLayout) -> FilePaneLayout {
    match l {
        PrefsFilePaneLayout::Single => FilePaneLayout::Single,
        PrefsFilePaneLayout::Dual => FilePaneLayout::Dual,
    }
}

pub fn view_mode_to_prefs(m: FileViewMode) -> PrefsFileViewMode {
    match m {
        FileViewMode::List => PrefsFileViewMode::List,
        FileViewMode::Details => PrefsFileViewMode::Details,
        FileViewMode::IconsSmall => PrefsFileViewMode::IconsSmall,
        FileViewMode::IconsLarge => PrefsFileViewMode::IconsLarge,
    }
}

pub fn pane_layout_to_prefs(l: FilePaneLayout) -> PrefsFilePaneLayout {
    match l {
        FilePaneLayout::Single => PrefsFilePaneLayout::Single,
        FilePaneLayout::Dual => PrefsFilePaneLayout::Dual,
    }
}

fn opt_columns(name_w: Option<f32>, size_w: Option<f32>) -> Option<FileDetailsColumns> {
    match (name_w, size_w) {
        (None, None) => None,
        (name_w, size_w) => Some(FileDetailsColumns {
            name_w: name_w.unwrap_or(240.0),
            size_w: size_w.unwrap_or(FileDetailsColumns::SIZE_DEFAULT),
        }),
    }
}

pub fn columns_from_ui_state(
    s: &FileManagerUiState,
    side: DetailsPaneSide,
) -> Option<FileDetailsColumns> {
    let (name_w, size_w) = match side {
        DetailsPaneSide::Left => (
            s.left_details_name_w.or(s.details_name_w),
            s.left_details_size_w.or(s.details_size_w),
        ),
        DetailsPaneSide::Right => (
            s.right_details_name_w.or(s.details_name_w),
            s.right_details_size_w.or(s.details_size_w),
        ),
    };
    opt_columns(name_w, size_w)
}

pub fn dual_split_from_ui_state(s: &FileManagerUiState) -> f32 {
    s.dual_split.unwrap_or(0.5).clamp(0.15, 0.85)
}

/// Wrap a file-manager session as workspace content (loads view prefs).
pub fn wrap_file_manager(mut s: FileManagerSession) -> Box<dyn WorkspaceContent> {
    let prefs = load_prefs();
    let show_hidden = prefs.file_manager.show_hidden;
    if let Some(left) = s.left_local.as_mut() {
        left.show_hidden = show_hidden;
    }
    if let Some(remote) = s.remote.as_mut() {
        remote.show_hidden = show_hidden;
    }
    s.right.show_hidden = show_hidden;
    let ui_fm = &prefs.ui_state.file_manager;
    Box::new(FileManagerContent {
        inner: s,
        pane_layout: prefs_to_pane_layout(prefs.file_manager.pane_layout),
        view_mode: prefs_to_view_mode(prefs.file_manager.view_mode),
        details_columns_left: columns_from_ui_state(ui_fm, DetailsPaneSide::Left),
        details_columns_right: columns_from_ui_state(ui_fm, DetailsPaneSide::Right),
        dual_split: dual_split_from_ui_state(ui_fm),
        search_panel_open: false,
        settings_menu: rsterm_uiframe::PopupMenuState::default(),
        hover_panel: rsterm_uiframe::hover_panel::HoverPanelState::default(),
        touch_multiselect: crate::page::touch_multiselect::TouchMultiselectState::default(),
        touch_ops_menu: rsterm_uiframe::PopupMenuState::default(),
        pending_open_settings: false,
        pending_prefs: None,
        pending_ui_state: None,
    })
}

impl WorkspaceContent for FileManagerContent {
    fn id(&self) -> &str {
        &self.inner.id
    }

    fn tab_label(&self) -> String {
        FileManagerSession::tab_label(&self.inner)
    }

    fn sidebar_has_new_window(&self) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut egui::Ui, ctx: &mut ContentUiCtx<'_>) -> ContentAction {
        let mut hamburger_clicked = false;
        let fm_action = {
            let mut on_hamburger = || {
                hamburger_clicked = true;
            };
            let mut chrome = PaneChrome {
                show_hamburger: ctx.show_hamburger,
                on_hamburger: &mut on_hamburger,
            };
            file_manager_view(
                ui,
                &mut self.inner,
                &mut self.pane_layout,
                &mut self.view_mode,
                &mut self.details_columns_left,
                &mut self.details_columns_right,
                &mut self.dual_split,
                &mut self.search_panel_open,
                &mut self.settings_menu,
                &mut self.hover_panel,
                &mut self.touch_multiselect,
                &mut self.touch_ops_menu,
                &mut self.pending_prefs,
                &mut self.pending_ui_state,
                &mut chrome,
            )
        };
        if hamburger_clicked {
            *ctx.hamburger_pending = true;
        }
        if fm_action.close {
            ContentAction::Close
        } else {
            if fm_action.open_settings {
                self.pending_open_settings = true;
            }
            ContentAction::None
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Persist FM view prefs to disk and return the snapshot for in-memory merge.
pub fn persist_file_manager_prefs(
    view_mode: FileViewMode,
    pane_layout: FilePaneLayout,
) -> FileManagerPrefs {
    persist_file_manager_prefs_full(
        view_mode,
        pane_layout,
        load_prefs().file_manager.show_hidden,
    )
}

/// Persist full FM prefs snapshot.
pub fn persist_file_manager_prefs_full(
    view_mode: FileViewMode,
    pane_layout: FilePaneLayout,
    show_hidden: bool,
) -> FileManagerPrefs {
    let mut prefs = load_prefs();
    prefs.file_manager = FileManagerPrefs {
        view_mode: view_mode_to_prefs(view_mode),
        pane_layout: pane_layout_to_prefs(pane_layout),
        show_hidden,
    };
    save_prefs(&prefs);
    prefs.file_manager
}

/// Persist one pane's Details column widths into `ui_state`.
pub fn persist_details_columns(
    side: DetailsPaneSide,
    cols: FileDetailsColumns,
) -> FileManagerUiState {
    let mut prefs = load_prefs();
    match side {
        DetailsPaneSide::Left => {
            prefs.ui_state.file_manager.left_details_name_w = Some(cols.name_w);
            prefs.ui_state.file_manager.left_details_size_w = Some(cols.size_w);
        }
        DetailsPaneSide::Right => {
            prefs.ui_state.file_manager.right_details_name_w = Some(cols.name_w);
            prefs.ui_state.file_manager.right_details_size_w = Some(cols.size_w);
        }
    }
    save_prefs(&prefs);
    prefs.ui_state.file_manager.clone()
}

/// Persist dual-pane split ratio into `ui_state`.
pub fn persist_dual_split(ratio: f32) -> FileManagerUiState {
    let mut prefs = load_prefs();
    prefs.ui_state.file_manager.dual_split = Some(ratio.clamp(0.15, 0.85));
    save_prefs(&prefs);
    prefs.ui_state.file_manager.clone()
}
