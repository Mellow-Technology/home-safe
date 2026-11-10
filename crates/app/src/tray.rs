//! The menu bar icon. It keeps Home Safe, and with it the API the extension
//! saves through, running after the window is closed.

use anyhow::{Context as _, Result};
use futures::StreamExt as _;
use futures::channel::mpsc;
use gpui_kit::{App, Global};
use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

/// Keeps the icon alive (dropping it removes it from the menu bar), plus the
/// menu item whose checkmark changes.
struct Tray {
    _icon: TrayIcon,
    start_at_login: CheckMenuItem,
}

impl Global for Tray {}

/// A choice from the icon's menu.
pub enum Command {
    Open,
    ToggleStartAtLogin,
    Quit,
}

/// Adds the icon. Choices from its menu go to `on_command`.
pub fn install(
    port: u16,
    start_at_login: bool,
    on_command: impl Fn(Command, &mut App) + 'static,
    cx: &mut App,
) -> Result<()> {
    let open_item = MenuItem::new("Open Home Safe", true, None);
    let api_item = MenuItem::new(format!("Saving homes on 127.0.0.1:{port}"), false, None);
    let login_item = CheckMenuItem::new("Start at Login", true, start_at_login, None);
    let quit_item = MenuItem::new("Quit Home Safe", true, None);
    let menu = Menu::new();
    menu.append_items(&[
        &open_item,
        &PredefinedMenuItem::separator(),
        &api_item,
        &PredefinedMenuItem::separator(),
        &login_item,
        &quit_item,
    ])?;

    let tray = TrayIconBuilder::new()
        .with_icon_templated(icon()?)
        .with_tooltip("Home Safe")
        .with_menu(Box::new(menu))
        .build()?;

    // Menu events arrive outside GPUI; hand them to a GPUI task.
    let (tx, mut rx) = mpsc::unbounded();
    let (open_id, login_id, quit_id) = (
        open_item.id().clone(),
        login_item.id().clone(),
        quit_item.id().clone(),
    );
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let command = if event.id == open_id {
            Command::Open
        } else if event.id == login_id {
            Command::ToggleStartAtLogin
        } else if event.id == quit_id {
            Command::Quit
        } else {
            return;
        };
        tx.unbounded_send(command).ok();
    }));
    cx.spawn(async move |cx| {
        while let Some(command) = rx.next().await {
            cx.update(|cx| on_command(command, cx));
        }
    })
    .detach();

    cx.set_global(Tray {
        _icon: tray,
        start_at_login: login_item,
    });
    Ok(())
}

/// Updates the Start at Login checkmark, if the icon is installed.
pub fn set_start_at_login(checked: bool, cx: &App) {
    if let Some(tray) = cx.try_global::<Tray>() {
        tray.start_at_login.set_checked(checked);
    }
}

/// The template icon from `scripts/make-icons.py`: macOS uses only its alpha
/// and tints it to match the menu bar.
fn icon() -> Result<Icon> {
    static PNG: &[u8] = include_bytes!("../assets/tray.png");
    let mut reader = png::Decoder::new(std::io::Cursor::new(PNG)).read_info()?;
    let mut rgba = vec![0; reader.output_buffer_size().context("tray icon too large")?];
    let info = reader.next_frame(&mut rgba)?;
    anyhow::ensure!(
        info.color_type == png::ColorType::Rgba,
        "tray icon must be RGBA"
    );
    rgba.truncate(info.buffer_size());
    Ok(Icon::from_rgba(rgba, info.width, info.height)?)
}
