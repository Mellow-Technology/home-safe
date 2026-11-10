//! Home Safe desktop app: saved homes as cards or a table, plus the local API the
//! browser extension saves listings through.
//!
//! On macOS the app lives in the menu bar: closing the window leaves it (and
//! the API) running, so the extension can still save homes. It can also
//! start at login, straight into the menu bar.

mod assets;
mod backend;
mod format;
mod homes_cards;
mod homes_table;
mod homes_view;
#[cfg(target_os = "macos")]
mod login_item;
#[cfg(target_os = "macos")]
mod tray;

use gpui_kit::*;

use crate::backend::Backend;
use crate::homes_view::HomesView;

actions!(home_safe, [Quit, CloseWindow, ToggleStartAtLogin]);

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,home_safe=debug".into()),
        )
        .init();

    let backend = Backend::start()?;

    let app = gpui_kit::application().with_assets(assets::AppAssets);
    // Clicking the Dock icon (shown only while a window is open) or launching
    // the app again brings the window back.
    let reopen = backend.clone();
    app.on_reopen(move |cx| show_window(&reopen, cx));
    app.run(move |cx| {
        gpui_kit::init(cx);
        set_dock_icon();
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.on_action(|_: &CloseWindow, cx| {
            if let Some(window) = cx.active_window() {
                window
                    .update(cx, |_, window, _| window.remove_window())
                    .ok();
            }
        });
        cx.bind_keys([
            KeyBinding::new("cmd-q", Quit, None),
            KeyBinding::new("cmd-w", CloseWindow, None),
        ]);
        homes_view::bind_keys(cx);
        set_menus(cx);

        // Launched at login: start in the menu bar, without the window.
        #[allow(unused_mut)]
        let mut background = false;
        #[cfg(target_os = "macos")]
        {
            cx.on_action(|_: &ToggleStartAtLogin, cx| toggle_start_at_login(cx));
            // Keep running in the menu bar with no window open; drop the
            // Dock icon meanwhile, like other menu bar apps.
            cx.set_quit_mode(QuitMode::Explicit);
            cx.on_window_closed(|cx, _| {
                if cx.windows().is_empty() {
                    cx.set_activation_policy(ActivationPolicy::Accessory);
                }
            })
            .detach();
            let open = backend.clone();
            let on_command = move |command, cx: &mut App| match command {
                tray::Command::Open => show_window(&open, cx),
                tray::Command::ToggleStartAtLogin => toggle_start_at_login(cx),
                tray::Command::Quit => cx.quit(),
            };
            match tray::install(backend.port, login_item::is_enabled(), on_command, cx) {
                Ok(()) => background = std::env::args().any(|a| a == login_item::BACKGROUND_FLAG),
                Err(err) => {
                    tracing::error!("menu bar icon unavailable: {err:#}");
                    // Without it there'd be no way back to a closed window.
                    cx.set_quit_mode(QuitMode::LastWindowClosed);
                }
            }
        }

        if background {
            #[cfg(target_os = "macos")]
            cx.set_activation_policy(ActivationPolicy::Accessory);
        } else {
            show_window(&backend, cx);
        }
    });
    Ok(())
}

fn set_menus(cx: &mut App) {
    let mut items = vec![];
    #[cfg(target_os = "macos")]
    items.extend([
        MenuItem::action("Start at Login", ToggleStartAtLogin).checked(login_item::is_enabled()),
        MenuItem::separator(),
    ]);
    items.extend([
        MenuItem::action("Close Window", CloseWindow),
        MenuItem::separator(),
        MenuItem::action("Quit Home Safe", Quit),
    ]);
    cx.set_menus(vec![Menu {
        name: "Home Safe".into(),
        items,
        disabled: false,
    }]);
}

/// Flips Start at Login, then syncs the checkmarks in both menus with what
/// actually happened.
#[cfg(target_os = "macos")]
fn toggle_start_at_login(cx: &mut App) {
    if let Err(err) = login_item::set_enabled(!login_item::is_enabled()) {
        tracing::error!("changing Start at Login: {err:#}");
        if let Some(window) = cx.windows().into_iter().next() {
            window
                .update(cx, |_, window, cx| {
                    let detail = format!("{err:#}");
                    // The alert stays up; only its answer (always OK) is dropped.
                    drop(window.prompt(
                        PromptLevel::Critical,
                        "Couldn't change Start at Login",
                        Some(&detail),
                        &["OK"],
                        cx,
                    ));
                })
                .ok();
        }
    }
    set_menus(cx);
    tray::set_start_at_login(login_item::is_enabled(), cx);
}

/// Brings the window to the front, opening it if it was closed.
fn show_window(backend: &Backend, cx: &mut App) {
    #[cfg(target_os = "macos")]
    cx.set_activation_policy(ActivationPolicy::Regular);
    if let Some(window) = cx.windows().into_iter().next() {
        window
            .update(cx, |_, window, _| window.activate_window())
            .ok();
    } else {
        open_window(backend.clone(), cx);
    }
    cx.activate(true);
}

fn open_window(backend: Backend, cx: &mut App) {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1280.), px(800.)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some("Home Safe".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| HomesView::new(backend, window, cx))
    })
    .expect("failed to open window");
}

/// Sets the Dock icon at runtime so it also shows for unbundled `cargo run`
/// builds; bundled builds get it from `AppIcon.icns` as well.
#[cfg(target_os = "macos")]
fn set_dock_icon() {
    use objc2::{AnyThread, MainThreadMarker};
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::NSData;

    static ICON: &[u8] = include_bytes!("../assets/icon.png");
    let Some(mtm) = MainThreadMarker::new() else { return };
    let data = NSData::with_bytes(ICON);
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        // SAFETY: on the main thread with a valid, non-nil image.
        unsafe { NSApplication::sharedApplication(mtm).setApplicationIconImage(Some(&image)) };
    }
}

#[cfg(not(target_os = "macos"))]
fn set_dock_icon() {}
