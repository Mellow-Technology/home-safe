//! Home Safe desktop app: saved homes as cards or a table, plus the local API the
//! browser extension saves listings through.

mod assets;
mod backend;
mod format;
mod homes_cards;
mod homes_table;
mod homes_view;

use gpui_kit::*;

use crate::backend::Backend;
use crate::homes_view::HomesView;

actions!(home_safe, [Quit]);

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,home_safe=debug".into()),
        )
        .init();

    let backend = Backend::start()?;

    gpui_kit::application()
        .with_assets(assets::AppAssets)
        .run(move |cx| {
            gpui_kit::init(cx);
            set_dock_icon();
            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
            homes_view::bind_keys(cx);
            cx.set_menus(vec![Menu {
                name: "Home Safe".into(),
                items: vec![MenuItem::action("Quit Home Safe", Quit)],
                disabled: false,
            }]);

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
            cx.activate(true);
        });
    Ok(())
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
