//! Auth user CRUD for Preferences → Users.

use super::RsTerminalApp;
use crate::persist::types::AuthUser;

impl RsTerminalApp {
    pub(crate) fn save_auth_user(&mut self, user: AuthUser) {
        if let Some(pos) = self.auth_users.iter().position(|u| u.id == user.id) {
            self.auth_users[pos] = user.clone();
        } else {
            self.auth_users.push(user.clone());
        }
        let _ = self.persist.upsert_auth_user(&user);
    }

    pub(crate) fn delete_auth_user(&mut self, id: &str) {
        self.auth_users.retain(|u| u.id != *id);
        let _ = self.persist.delete_auth_user(id);
        // Clear dangling refs on connections.
        for conn in &mut self.saved_connections {
            if conn.auth_user_id.as_deref() == Some(id) {
                conn.auth_user_id = None;
                let _ = self.persist.upsert_connection(conn);
            }
        }
    }
}
