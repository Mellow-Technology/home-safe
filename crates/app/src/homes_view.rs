//! The app's root view: the list of homes (cards or table), a page for one
//! home, and a full-window photo viewer over that page.

mod detail;
mod viewer;

use std::path::PathBuf;

use gpui_kit::component::button::{Button, ButtonGroup};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::status_bar::StatusBar;
use gpui_kit::component::table::{DataTable, TableEvent, TableState};
use gpui_kit::component::{ActiveTheme as _, IconName, Selectable as _, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use home_safe_core::{Home, HomeImage};
use home_safe_server::StoreEvent;

use crate::backend::Backend;
use crate::homes_cards;
use crate::homes_table::HomesTable;

actions!(home_safe, [GoBack, PrevPhoto, NextPhoto, ClosePhoto]);

/// The app icon without macOS margins, for the toolbar.
static LOGO: std::sync::LazyLock<std::sync::Arc<Image>> = std::sync::LazyLock::new(|| {
    std::sync::Arc::new(Image::from_bytes(ImageFormat::Png, include_bytes!("../assets/logo.png").to_vec()))
});

const DETAIL_CONTEXT: &str = "HomeDetail";
const VIEWER_CONTEXT: &str = "PhotoViewer";

/// Keyboard shortcuts for the home page and the photo viewer.
pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("escape", GoBack, Some(DETAIL_CONTEXT)),
        KeyBinding::new("left", PrevPhoto, Some(VIEWER_CONTEXT)),
        KeyBinding::new("right", NextPhoto, Some(VIEWER_CONTEXT)),
        KeyBinding::new("escape", ClosePhoto, Some(VIEWER_CONTEXT)),
    ]);
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ViewMode {
    #[default]
    Cards,
    Table,
}

pub struct HomesView {
    backend: Backend,
    mode: ViewMode,
    table: Entity<TableState<HomesTable>>,
    search: Entity<InputState>,
    cards_scroll: ScrollHandle,
    /// Home highlighted in the list: the last one opened or saved.
    highlighted: Option<i64>,
    /// The open home page, if any.
    detail: Option<Detail>,
    /// Index into `detail.photos()` shown in the photo viewer.
    viewer: Option<usize>,
    detail_focus: FocusHandle,
    viewer_focus: FocusHandle,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

struct Detail {
    home: Home,
    images: Vec<HomeImage>,
}

impl Detail {
    /// Downloaded photos, in listing order, with their captions.
    fn photos(&self) -> Vec<(PathBuf, &HomeImage)> {
        self.images
            .iter()
            .filter_map(|i| i.local_path.as_ref().map(|p| (PathBuf::from(p), i)))
            .collect()
    }
}

impl HomesView {
    pub fn new(backend: Backend, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let this = cx.weak_entity();
        let delegate = HomesTable::new().on_delete(move |home, window, cx| {
            this.update(cx, |this, cx| this.delete_home(home, window, cx)).ok();
        });
        let table = cx.new(|cx| TableState::new(delegate, window, cx));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search address, city, type…"));

        let subscriptions = vec![
            cx.subscribe_in(&table, window, |this, table, event, window, cx| match event {
                TableEvent::SelectRow(row) => {
                    this.highlighted = table.read(cx).delegate().home(*row).map(|h| h.id);
                }
                TableEvent::DoubleClickedRow(row) => {
                    if let Some(home) = table.read(cx).delegate().home(*row).cloned() {
                        this.open_home(home, window, cx);
                    }
                }
                _ => {}
            }),
            cx.subscribe(&search, |this, search, event, cx| {
                if let InputEvent::Change = event {
                    let query = search.read(cx).value();
                    this.table.update(cx, |t, cx| {
                        t.delegate_mut().set_filter(&query);
                        t.refresh(cx);
                    });
                }
            }),
        ];

        let mut this = Self {
            backend,
            mode: ViewMode::default(),
            table,
            search,
            cards_scroll: ScrollHandle::new(),
            highlighted: None,
            detail: None,
            viewer: None,
            detail_focus: cx.focus_handle(),
            viewer_focus: cx.focus_handle(),
            error: None,
            _subscriptions: subscriptions,
        };
        this.listen_for_changes(cx);
        this.reload(cx);
        this
    }

    /// Reloads whenever the API server changes the database.
    fn listen_for_changes(&mut self, cx: &mut Context<Self>) {
        let mut events = self.backend.events.subscribe();
        cx.spawn(async move |this, cx| {
            loop {
                let event = match events.recv().await {
                    Ok(event) => Some(event),
                    // Missed some events: a full reload covers them.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => None,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                };
                let alive = this.update(cx, |this, cx| {
                    if let Some(StoreEvent::HomeSaved(id)) = event {
                        this.highlighted = Some(id);
                    }
                    this.reload(cx);
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    /// Reloads the list, and the open home page if there is one; closes the
    /// page if its home was deleted.
    fn reload(&mut self, cx: &mut Context<Self>) {
        let store = self.backend.store.clone();
        let load = self.backend.run(async move { store.list_homes().await });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            this.update(cx, |this, cx| match result {
                Ok(homes) => {
                    this.error = None;
                    if let Some(detail) = &mut this.detail {
                        match homes.iter().find(|h| h.id == detail.home.id) {
                            Some(home) => {
                                detail.home = home.clone();
                                let id = home.id;
                                this.load_images(id, cx);
                            }
                            None => {
                                this.detail = None;
                                this.viewer = None;
                            }
                        }
                    }
                    let highlighted = this.highlighted;
                    this.table.update(cx, |t, cx| {
                        t.delegate_mut().set_homes(homes);
                        if let Some(row) = highlighted.and_then(|id| t.delegate().row_of(id)) {
                            t.set_selected_row(row, cx);
                        }
                        t.refresh(cx);
                    });
                    cx.notify();
                }
                Err(err) => {
                    this.error = Some(format!("Failed to load homes: {err:#}").into());
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn open_home(&mut self, home: Home, window: &mut Window, cx: &mut Context<Self>) {
        let id = home.id;
        // Keep the photos already loaded if this home is reopened.
        let images = match self.detail.take() {
            Some(d) if d.home.id == id => d.images,
            _ => Vec::new(),
        };
        self.highlighted = Some(id);
        self.detail = Some(Detail { home, images });
        self.viewer = None;
        window.focus(&self.detail_focus, cx);
        self.load_images(id, cx);
        cx.notify();
    }

    fn load_images(&mut self, id: i64, cx: &mut Context<Self>) {
        let store = self.backend.store.clone();
        let load = self.backend.run(async move { store.images(id).await });
        cx.spawn(async move |this, cx| {
            if let Ok(images) = load.await {
                this.update(cx, |this, cx| {
                    let Some(detail) = this.detail.as_mut().filter(|d| d.home.id == id) else { return };
                    detail.images = images;
                    let count = detail.photos().len();
                    this.viewer = this.viewer.filter(|_| count > 0).map(|ix| ix.min(count - 1));
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    fn go_back(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.detail = None;
        self.viewer = None;
        cx.notify();
    }

    fn open_photo(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.viewer = Some(ix);
        window.focus(&self.viewer_focus, cx);
        cx.notify();
    }

    fn close_photo(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.viewer = None;
        window.focus(&self.detail_focus, cx);
        cx.notify();
    }

    /// Moves the viewer by `delta` photos, wrapping around at either end.
    fn step_photo(&mut self, delta: isize, cx: &mut Context<Self>) {
        let count = self.detail.as_ref().map_or(0, |d| d.photos().len());
        if let (Some(ix), true) = (self.viewer, count > 0) {
            self.viewer = Some(wrap(ix, delta, count));
            cx.notify();
        }
    }

    fn delete_home(&mut self, home: Home, window: &mut Window, cx: &mut Context<Self>) {
        let answer = window.prompt(
            PromptLevel::Warning,
            &format!("Delete {}?", home.full_address()),
            Some("This removes the home, its saved data and its downloaded photos from Home Safe."),
            &["Delete", "Cancel"],
            cx,
        );
        let images = self.backend.images.clone();
        let events = self.backend.events.clone();
        let backend = self.backend.clone();
        cx.spawn(async move |this, cx| {
            if answer.await.ok() != Some(0) {
                return;
            }
            let result = backend
                .run(async move {
                    images.delete_home(home.id).await?;
                    let _ = events.send(StoreEvent::HomeDeleted(home.id));
                    Ok(())
                })
                .await;
            if let Err(err) = result {
                this.update(cx, |this, cx| {
                    this.error = Some(format!("Failed to delete home: {err:#}").into());
                    cx.notify();
                })
                .ok();
            }
        })
        .detach();
    }

    /// Logo and title on the left, search centered in the window, view switch on the right.
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        h_flex()
            .gap_3()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .flex_1()
                    .gap_2()
                    .child(img(LOGO.clone()).size(px(24.)).flex_shrink_0())
                    .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Home Safe")),
            )
            .child(
                div().w(px(400.)).child(
                    Input::new(&self.search)
                        .small()
                        .prefix(IconName::Search)
                        .cleanable(true),
                ),
            )
            .child(
                h_flex().flex_1().justify_end().child(
                    ButtonGroup::new("view-mode")
                        .small()
                        .outline()
                        .child(
                            Button::new("view-cards")
                                .icon(gpui_kit::assets::IconName::LayoutGrid)
                                .tooltip("Card view")
                                .selected(self.mode == ViewMode::Cards),
                        )
                        .child(
                            Button::new("view-table")
                                .icon(gpui_kit::assets::IconName::Table)
                                .tooltip("Table view")
                                .selected(self.mode == ViewMode::Table),
                        )
                        .on_click(cx.listener(|this, clicked: &Vec<usize>, _, cx| {
                            this.mode = if clicked.contains(&1) { ViewMode::Table } else { ViewMode::Cards };
                            cx.notify();
                        })),
                ),
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let table = self.table.read(cx).delegate();
        let (shown, total) = (table.visible().count(), table.total());
        let plural = if total == 1 { "home" } else { "homes" };
        let count = if shown == total {
            format!("{total} {plural}")
        } else {
            format!("{shown} of {total} {plural}")
        };
        StatusBar::new()
            .left(count)
            .right(format!("API · 127.0.0.1:{}", self.backend.port))
            .px_4()
    }

    fn render_cards(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let table = self.table.read(cx).delegate();
        let homes: Vec<Home> = table.visible().cloned().collect();
        if homes.is_empty() {
            return h_flex()
                .size_full()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child(table.empty_message())
                .into_any_element();
        }

        let width = window.viewport_size().width - px(2. * homes_cards::PADDING);
        let cards = homes.into_iter().enumerate().map(|(row, home)| {
            let highlighted = Some(home.id) == self.highlighted;
            let on_delete = cx.weak_entity();
            let open = home.clone();
            homes_cards::card(row, &home, highlighted, cx)
                .min_w_0()
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.open_home(open.clone(), window, cx);
                }))
                .context_menu(move |menu, _, _| {
                    let on_delete = on_delete.clone();
                    let home = home.clone();
                    menu.item(PopupMenuItem::new("Delete…").icon(gpui_kit::assets::IconName::Trash).on_click(
                        move |_, window, cx| {
                            on_delete
                                .update(cx, |this, cx| this.delete_home(home.clone(), window, cx))
                                .ok();
                        },
                    ))
                })
        });

        homes_cards::grid(homes_cards::columns(width), cards)
            .track_scroll(&self.cards_scroll)
            .into_any_element()
    }

    fn render_list(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let content = match self.mode {
            ViewMode::Cards => self.render_cards(window, cx),
            ViewMode::Table => DataTable::new(&self.table).stripe(true).into_any_element(),
        };
        v_flex()
            .size_full()
            .child(self.render_toolbar(cx))
            .child(div().flex_1().min_h_0().child(content))
    }
}

impl Render for HomesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let page = match &self.detail {
            Some(detail) => self.render_detail(detail, window, cx).into_any_element(),
            None => self.render_list(window, cx).into_any_element(),
        };
        let viewer = self
            .detail
            .as_ref()
            .zip(self.viewer)
            .map(|(detail, ix)| self.render_viewer(detail, ix, window, cx));

        v_flex()
            .relative()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .when_some(self.error.clone(), |this, error| {
                this.child(
                    div()
                        .px_4()
                        .py_2()
                        .text_sm()
                        .text_color(cx.theme().danger)
                        .child(error),
                )
            })
            .child(div().flex_1().min_h_0().child(page))
            .child(self.render_status_bar(cx))
            .children(viewer)
    }
}

/// `ix + delta`, wrapping around within `0..count`.
fn wrap(ix: usize, delta: isize, count: usize) -> usize {
    (ix as isize + delta).rem_euclid(count as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn photo_navigation_wraps() {
        assert_eq!(wrap(0, 1, 3), 1);
        assert_eq!(wrap(2, 1, 3), 0);
        assert_eq!(wrap(0, -1, 3), 2);
        assert_eq!(wrap(0, 1, 1), 0);
    }
}
