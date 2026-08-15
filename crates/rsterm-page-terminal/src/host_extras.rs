//! Host extras for terminal pane content `ui`.
//!
//! Uses raw pointers so the type is `'static` and can be downcast via [`std::any::Any`].
//! Pointers are only valid for the duration of a single content `ui` call.

use rsterm_data::persist::types::TerminalProfile;
use rsterm_uiframe::keyboard::VirtualKeyboard;

/// Terminal-specific host context passed via [`rsterm_workspace::ContentUiCtx::extras`].
pub struct TerminalHostExtras {
    profiles: *const [TerminalProfile],
    virtual_keyboard: *mut VirtualKeyboard,
}

impl TerminalHostExtras {
    /// Build extras from live host borrows for one content `ui` call.
    ///
    /// The returned value (and any `Any` downcast of it) must not outlive those borrows.
    pub fn new(profiles: &[TerminalProfile], virtual_keyboard: &mut VirtualKeyboard) -> Self {
        Self {
            profiles: profiles as *const [TerminalProfile],
            virtual_keyboard: virtual_keyboard as *mut VirtualKeyboard,
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

    /// Borrow profiles and keyboard for one content `ui` call.
    pub fn split_mut(&mut self) -> (&[TerminalProfile], &mut VirtualKeyboard) {
        // SAFETY: distinct pointers; both valid for this render.
        unsafe { (&*self.profiles, &mut *self.virtual_keyboard) }
    }
}
