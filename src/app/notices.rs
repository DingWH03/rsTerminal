//! App-level notice flags: when to show quit confirm / connection failure.

use super::RsTerminalApp;
use crate::ui::page::dialogs::{paint_connection_notice, paint_quit_confirm};

impl RsTerminalApp {
    pub(crate) fn paint_notices(&mut self, ctx: &egui::Context) {
        paint_connection_notice(ctx, &mut self.connection_notice);

        let session_count = self.sessions.len();
        if paint_quit_confirm(ctx, &mut self.show_quit_dialog, session_count) {
            self.quit_after_close = true;
            self.close_all_sessions();
            self.request_app_exit(ctx);
        }
    }
}
