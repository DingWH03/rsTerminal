//! Host extras for [`crate::ContentUiCtx::extras`].
//!
//! Uses raw pointers so the type is `'static` and can be downcast via [`std::any::Any`].
//! Pointers are only valid for the duration of a single content `ui` call.

use rsterm_data::persist::types::TerminalProfile;
use rsterm_uiframe::keyboard::VirtualKeyboard;

/// Shared pane-host context passed to every [`crate::WorkspaceContent`] `ui` call.
pub struct PaneHostExtras {
    profiles: *const [TerminalProfile],
    virtual_keyboard: *mut VirtualKeyboard,
    pub show_hamburger: bool,
    hamburger_pending: *mut bool,
    pub pane_focus_click: bool,
}

impl PaneHostExtras {
    /// Build extras from live host borrows for one content `ui` call.
    ///
    /// The returned value (and any `Any` downcast of it) must not outlive those borrows.
    pub fn new(
        profiles: &[TerminalProfile],
        virtual_keyboard: &mut VirtualKeyboard,
        show_hamburger: bool,
        hamburger_pending: &mut bool,
    ) -> Self {
        Self {
            profiles: profiles as *const [TerminalProfile],
            virtual_keyboard: virtual_keyboard as *mut VirtualKeyboard,
            show_hamburger,
            hamburger_pending: hamburger_pending as *mut bool,
            pane_focus_click: false,
        }
    }

    pub fn profiles(&self) -> &[TerminalProfile] {
        // SAFETY: constructed from a live borrow in the host render path.
        unsafe { &*self.profiles }
    }

    pub fn virtual_keyboard(&mut self) -> &mut VirtualKeyboard {
        // SAFETY: constructed from a live borrow in the host render path.
        unsafe { &mut *self.virtual_keyboard }
    }

    /// Mark that the content hamburger was clicked (host calls `hamburger_click` after `ui`).
    pub fn request_hamburger(&mut self) {
        // SAFETY: constructed from a live borrow in the host render path.
        unsafe {
            *self.hamburger_pending = true;
        }
    }

    /// Borrow profiles, keyboard, and pane-focus flag for one content `ui` call.
    pub fn split_mut(&mut self) -> (&[TerminalProfile], &mut VirtualKeyboard, &mut bool) {
        // SAFETY: distinct pointers + a field; all valid for this render.
        unsafe {
            (
                &*self.profiles,
                &mut *self.virtual_keyboard,
                &mut self.pane_focus_click,
            )
        }
    }
}
