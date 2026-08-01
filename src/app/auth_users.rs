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
        match self.persist.delete_auth_user(id) {
            Ok(()) => {
                self.auth_users.retain(|u| u.id != *id);
            }
            Err(e) if e.starts_with("auth_user_in_use:") => {
                let n = e.strip_prefix("auth_user_in_use:").unwrap_or("?");
                self.connection_notice =
                    Some(rust_i18n::t!("err_auth_user_in_use", count = n).into_owned());
            }
            Err(e) => {
                self.connection_notice = Some(e);
            }
        }
    }
}
