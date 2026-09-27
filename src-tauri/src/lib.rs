mod commands;
mod engine;
mod indexing;
mod settings;

use engine::Engine;
use settings::Settings;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

pub const NAVIGATE_EVENT: &str = "trove://navigate";

pub struct AppState {
    pub engine: Arc<Engine>,
    settings: Mutex<Settings>,
    settings_path: PathBuf,
}

impl AppState {
    pub fn settings(&self) -> MutexGuard<'_, Settings> {
        self.settings.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Apply `f` and persist. Nothing is saved if `f` fails.
    pub fn update_settings<T>(
        &self,
        f: impl FnOnce(&mut Settings) -> Result<T, String>,
    ) -> Result<T, String> {
        let mut settings = self.settings();
        let mut next = settings.clone();
        let out = f(&mut next)?;
        next.save(&self.settings_path)
            .map_err(|e| format!("{e:#}"))?;
        *settings = next;
        Ok(out)
    }
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.unminimize();
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Visible means hide, hidden means show. Focus isn't consulted: Windows'
/// foreground lock can refuse `set_focus`, and the frontend already hides the
/// window when it loses focus.
fn toggle_main(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else {
        return;
    };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
    } else {
        show_main(app);
    }
}

fn navigate(app: &AppHandle, view: &str) {
    show_main(app);
    let _ = app.emit(NAVIGATE_EVENT, view);
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Trove", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Folders & settings…", true, None::<&str>)?;
    let reindex = MenuItem::with_id(app, "reindex", "Re-index now", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Trove", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open, &settings, &reindex, &sep, &quit])?;

    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Trove")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => navigate(app, "search"),
            "settings" => navigate(app, "settings"),
            "reindex" => indexing::start(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let paths = app.path();
    let settings_path = paths.app_config_dir()?.join("settings.json");
    let engine = Arc::new(Engine::open(&paths.app_local_data_dir()?)?);
    let settings = Settings::load(&settings_path);
    let shortcut = settings.shortcut.clone();

    app.manage(AppState {
        engine: engine.clone(),
        settings: Mutex::new(settings),
        settings_path,
    });

    build_tray(app)?;

    // A shortcut already taken by another app shouldn't stop Trove starting;
    // the tray still opens it.
    if let Err(e) = app
        .global_shortcut()
        .on_shortcut(shortcut.as_str(), |app, _, event| {
            if event.state == ShortcutState::Pressed {
                tracing::debug!("global shortcut pressed");
                toggle_main(app);
            }
        })
    {
        tracing::warn!("could not register shortcut {shortcut}: {e}");
    }

    // Load the model, then bring the index up to date with any changes made
    // while Trove was closed.
    let handle = app.handle().clone();
    std::thread::spawn(move || {
        if let Err(e) = engine.warm_up() {
            tracing::error!("model warm-up failed: {e:#}");
        }
        indexing::start(&handle);
    });
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Logs go to stderr; `RUST_LOG=debug` for more (release builds have no
    // console window, so this is for `tauri dev` and terminal launches).
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tantivy=warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    tauri::Builder::default()
        // Must be registered first: a second launch just focuses this one.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            show_main(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(setup)
        .on_window_event(|window, event| {
            // Closing hides to the tray; "Quit Trove" in the tray exits.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_state,
            commands::add_folder,
            commands::remove_folder,
            commands::reindex,
            commands::search,
            commands::open_file,
            commands::reveal_file,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Trove");
}
