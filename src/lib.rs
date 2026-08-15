//! rsTerminal library (desktop `bin` + Android `cdylib`).

// The crate name intentionally uses CamelCase to match the project name.
#![allow(non_snake_case)]
#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]

// Load all locale files from the `locales` directory at compile time.
rust_i18n::i18n!("locales", fallback = "en");

pub mod app;
pub mod fonts;
pub mod i18n;
pub mod session;
pub mod ui;

pub use rsterm_config as config;
pub use rsterm_connection as connection;
pub use rsterm_data as data;
pub use rsterm_fs as fs;
pub use rsterm_platform as platform;
pub use rsterm_remote as remote;
pub use rsterm_terminal as terminal;
pub use rsterm_uiframe as uiframe;
pub use rsterm_workspace as workspace;

use app::RsTerminalApp;
use log::info;

fn install_shell_host_hooks() {
    use std::sync::Arc;

    rsterm_shell::install_host_hooks(rsterm_shell::HostHooks {
        monospace_catalog_status: || match fonts::monospace_catalog_status() {
            fonts::MonospaceCatalogStatus::Loading => rsterm_shell::FontCatalogStatus::Loading,
            fonts::MonospaceCatalogStatus::Ready(entries) => {
                rsterm_shell::FontCatalogStatus::Ready(Arc::new(
                    entries
                        .iter()
                        .map(|e| rsterm_shell::FontEntry {
                            path: e.path.clone(),
                            label: e.label.clone(),
                        })
                        .collect(),
                ))
            }
        },
        apply_terminal_fonts: fonts::apply_terminal_fonts,
        apply_language: i18n::apply_language,
        apply_ui_theme: i18n::apply_ui_theme,
        language_label: i18n::language_label,
        ui_theme_label: i18n::ui_theme_label,
        cursor_style_label: i18n::cursor_style_label,
    });
}

pub fn run_app(native_options: eframe::NativeOptions) {
    if let Err(e) = eframe::run_native(
        "rsTerminal",
        native_options,
        Box::new(|cc| {
            let persist = crate::data::Persist::open();
            let app = RsTerminalApp::new(persist);
            fonts::setup_fonts(&cc.egui_ctx, app.default_terminal_font());
            fonts::preload_monospace_catalog();
            fonts::tune_android_display(&cc.egui_ctx);
            rsterm_page_terminal::fonts::install_font_hooks(
                rsterm_page_terminal::fonts::FontHooks {
                    font_generation: fonts::font_generation,
                },
            );
            install_shell_host_hooks();
            // Desktop: real OS child windows for DialogFrame.
            // Android keeps embedding (single Activity window).
            #[cfg(not(target_os = "android"))]
            cc.egui_ctx.set_embed_viewports(false);
            Ok(Box::new(app))
        }),
    ) {
        info!("Failed to start: {e}");
    }
}

/// Desktop entry.
#[cfg(not(target_os = "android"))]
pub fn run_desktop() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    crate::platform::init_platform();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title(rust_i18n::t!("app_title")),
        centered: true,
        ..Default::default()
    };

    run_app(native_options);
}

/// Serialises `android_main` so that the previous eframe event-loop is fully
/// torn down before a new one starts (Android may recreate the Activity in
/// the same process while the old thread is still shutting down).
#[cfg(target_os = "android")]
static ANDROID_MAIN_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Android entry (winit / eframe). Loaded via `NativeActivity` + `android.app.lib_name`.
///
/// When Android recreates the Activity in the same process (user presses back
/// and re-launches), a new thread is spawned for `android_main` while the
/// previous one may still be shutting down.  The `ANDROID_MAIN_GUARD` mutex
/// ensures only one instance runs at a time, preventing eframe / winit state
/// conflicts.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    // ── Drain any previous eframe event-loop before starting a new one ──
    let _guard = ANDROID_MAIN_GUARD.lock().unwrap();

    android_logger::init_once(
        android_logger::Config::default().with_max_level(log::LevelFilter::Info),
    );

    // -- Initialise platform trait + Android services --------------------
    platform::init_android_platform(&app);
    // -- Back‑button interception via custom NativeActivity -----
    platform::android_back::init(&app);

    // -- Initialise persistent config path ------------------------------
    // Android NativeActivity does NOT set $HOME / $XDG_CONFIG_HOME, so
    // the `directories` crate returns None and config is lost on restart.
    // Use the app-internal data path provided by the system instead.
    if let Some(data_dir) = app.internal_data_path() {
        crate::data::init_android_base_dir(data_dir);
        log::info!("Android config dir: {:?}", crate::data::config_dir());
    }

    let native_options = eframe::NativeOptions {
        android_app: Some(app),
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1024.0, 768.0])
            .with_title("rsTerminal"),
        ..Default::default()
    };

    run_app(native_options);
}
