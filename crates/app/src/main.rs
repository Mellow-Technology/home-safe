//! Home Safe desktop app: a table of saved homes, plus the local API the
//! browser extension saves listings through.

mod backend;
mod format;
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

    gpui_kit::application().run(move |cx| {
        gpui_kit::init(cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
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
