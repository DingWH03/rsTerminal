//! Favorite command CRUD and injection into the focused terminal.

use super::RsTerminalApp;
use crate::data::persist::types::FavoriteCommand;
use crate::session::{ConnectionViewAction, WorkspaceSession};
use crate::ui::page::terminal::paste_to_session;

impl RsTerminalApp {
    pub(crate) fn save_favorite_command(&mut self, mut cmd: FavoriteCommand) {
        if let Some(pos) = self.favorite_commands.iter().position(|c| c.id == cmd.id) {
            if cmd.sort_order == 0 {
                cmd.sort_order = self.favorite_commands[pos].sort_order;
            }
            self.favorite_commands[pos] = cmd.clone();
        } else {
            if cmd.sort_order == 0 {
                cmd.sort_order = self
                    .favorite_commands
                    .iter()
                    .map(|c| c.sort_order)
                    .max()
                    .unwrap_or(0)
                    + 1;
            }
            self.favorite_commands.push(cmd.clone());
        }
        let _ = self.persist.upsert_command(&cmd);
    }

    pub(crate) fn delete_favorite_command(&mut self, id: &str) {
        self.favorite_commands.retain(|c| c.id != *id);
        let _ = self.persist.delete_command(id);
    }

    /// Insert the command into the focused terminal; append CR when `auto_execute`.
    pub(crate) fn run_favorite_command(&mut self, id: &str, ctx: &egui::Context) {
        let Some(cmd) = self.favorite_commands.iter().find(|c| c.id == id).cloned() else {
            return;
        };
        let Some(idx) = self.focused_session_index() else {
            return;
        };
        let WorkspaceSession::Terminal(term) = &mut self.sessions[idx] else {
            return;
        };
        let mut noop = ConnectionViewAction::None;
        paste_to_session(term, &cmd.command, ctx, &mut noop);
        if cmd.auto_execute {
            term.send_active(b"\r".to_vec());
            let _ = crate::session::drain_connection(term, &mut noop);
            ctx.request_repaint();
        }
    }
}
