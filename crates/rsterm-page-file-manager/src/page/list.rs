use rsterm_session_core::{FilePaneState, RemotePane};

pub(super) fn dismiss_multiselect_local(pane: &mut FilePaneState) {
    pane.select_mode = false;
    pane.selected.clear();
}

pub(super) fn dismiss_multiselect_remote(remote: &mut RemotePane) {
    remote.select_mode = false;
    remote.selected.clear();
}
