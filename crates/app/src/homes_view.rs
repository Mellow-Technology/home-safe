use std::path::PathBuf;

use gpui_kit::component::button::{Button, ButtonGroup, ButtonVariants as _};
use gpui_kit::component::menu::{ContextMenuExt as _, PopupMenuItem};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::table::{DataTable, TableEvent, TableState};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Selectable as _, Sizable as _, h_flex, v_flex,
};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use home_safe_core::{Home, HomeImage};
use home_safe_server::StoreEvent;

use crate::backend::Backend;
use crate::format;
use crate::homes_cards;
use crate::homes_table::HomesTable;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum ViewMode {
    #[default]
    Cards,
    Table,
}

/// Width of the detail panel shown beside the homes.
const DETAIL_WIDTH: f32 = 380.;

pub struct HomesView {
    backend: Backend,
    mode: ViewMode,
    table: Entity<TableState<HomesTable>>,
    search: Entity<InputState>,
    selected: Option<Selected>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

struct Selected {
    home: Home,
    images: Vec<HomeImage>,
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
            cx.subscribe_in(&table, window, |this, table, event, _, cx| match event {
                TableEvent::SelectRow(row) => {
                    let home = table.read(cx).delegate().home(*row).cloned();
                    this.select(home, cx);
                }
                TableEvent::DoubleClickedRow(row) => {
                    if let Some(url) = table.read(cx).delegate().home(*row).and_then(|h| h.url.clone()) {
                        cx.open_url(&url);
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
            selected: None,
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
                        this.reload_and_select(Some(id), cx);
                    } else {
                        this.reload(cx);
                    }
                });
                if alive.is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        let selected = self.selected.as_ref().map(|s| s.home.id);
        self.reload_and_select(selected, cx);
    }

    fn reload_and_select(&mut self, select_id: Option<i64>, cx: &mut Context<Self>) {
        let store = self.backend.store.clone();
        let load = self.backend.run(async move { store.list_homes().await });
        cx.spawn(async move |this, cx| {
            let result = load.await;
            this.update(cx, |this, cx| match result {
                Ok(homes) => {
                    this.error = None;
                    this.table.update(cx, |t, cx| {
                        t.delegate_mut().set_homes(homes);
                        t.refresh(cx);
                    });
                    let home = select_id.and_then(|id| {
                        let t = this.table.read(cx);
                        let row = t.delegate().row_of(id)?;
                        Some((row, t.delegate().home(row)?.clone()))
                    });
                    match home {
                        Some((row, home)) => {
                            this.table.update(cx, |t, cx| t.set_selected_row(row, cx));
                            this.select(Some(home), cx);
                        }
                        None => this.select(None, cx),
                    }
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

    fn select(&mut self, home: Option<Home>, cx: &mut Context<Self>) {
        let Some(home) = home else {
            self.selected = None;
            cx.notify();
            return;
        };
        let store = self.backend.store.clone();
        let id = home.id;
        // Keep showing the previous photos until the new ones load if the
        // same home is reselected (e.g. after an image download finishes).
        let images = match &self.selected {
            Some(s) if s.home.id == id => std::mem::take(&mut self.selected.as_mut().unwrap().images),
            _ => Vec::new(),
        };
        self.selected = Some(Selected { home, images });
        cx.notify();

        let load = self.backend.run(async move { store.images(id).await });
        cx.spawn(async move |this, cx| {
            if let Ok(images) = load.await {
                this.update(cx, |this, cx| {
                    if let Some(s) = this.selected.as_mut().filter(|s| s.home.id == id) {
                        s.images = images;
                        cx.notify();
                    }
                })
                .ok();
            }
        })
        .detach();
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

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let total = self.table.read(cx).delegate().total();
        h_flex()
            .gap_3()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(div().text_lg().font_weight(FontWeight::SEMIBOLD).child("Home Safe"))
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("{total} saved")),
            )
            .child(div().flex_1())
            .child(
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
            )
            .child(
                div().w(px(320.)).child(
                    Input::new(&self.search)
                        .small()
                        .prefix(IconName::Search)
                        .cleanable(true),
                ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("API · 127.0.0.1:{}", self.backend.port)),
            )
    }

    fn render_detail(&self, selected: &Selected, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let home = &selected.home;
        let theme = cx.theme();
        let photo_paths: Vec<PathBuf> = selected
            .images
            .iter()
            .filter_map(|i| i.local_path.as_ref().map(PathBuf::from))
            .collect();
        let pending = selected.images.len() - photo_paths.len();

        let fact = |label: &'static str, value: String| {
            v_flex()
                .min_w(px(90.))
                .child(div().text_xs().text_color(theme.muted_foreground).child(label))
                .child(div().text_sm().child(if value.is_empty() { "—".into() } else { value }))
        };

        let facts = h_flex()
            .flex_wrap()
            .gap_3()
            .child(fact("Price", format::money(home.price)))
            .child(fact("Zestimate", format::money(home.zestimate)))
            .child(fact("Beds", format::decimal(home.beds)))
            .child(fact("Baths", format::decimal(home.baths)))
            .child(fact("Sq Ft", format::number(home.sqft)))
            .child(fact("Lot Sq Ft", format::number(home.lot_sqft)))
            .child(fact("Built", home.year_built.map(|y| y.to_string()).unwrap_or_default()))
            .child(fact("HOA/mo", format::money(home.hoa_monthly)))
            .child(fact("Rent Zestimate", format::money(home.rent_zestimate)))
            .child(fact("Days on Zillow", format::number(home.days_on_market)));

        let url = home.url.clone();
        let photos_dir = self.backend.images_dir.join(home.id.to_string());

        v_flex()
            .id("detail")
            .w(px(DETAIL_WIDTH))
            .h_full()
            .flex_shrink_0()
            .border_l_1()
            .border_color(theme.border)
            .overflow_y_scroll()
            .p_4()
            .gap_3()
            .when_some(photo_paths.first().cloned(), |this, cover| {
                this.child(
                    img(cover)
                        .w_full()
                        .h(px(220.))
                        .rounded(px(6.))
                        .object_fit(ObjectFit::Cover),
                )
            })
            .child(
                v_flex()
                    .child(div().text_base().font_weight(FontWeight::SEMIBOLD).child(
                        home.address.clone().unwrap_or_else(|| format!("zpid {}", home.source_id)),
                    ))
                    .child(div().text_sm().text_color(theme.muted_foreground).child(
                        [home.city.as_deref(), home.state.as_deref(), home.zip.as_deref()]
                            .into_iter()
                            .flatten()
                            .collect::<Vec<_>>()
                            .join(" "),
                    ))
                    .child(div().text_xs().text_color(theme.muted_foreground).child(format!(
                        "{} · {}",
                        format::humanize(home.home_type.as_deref()),
                        format::humanize(home.status.as_deref())
                    ))),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("open-listing")
                            .small()
                            .primary()
                            .icon(IconName::ExternalLink)
                            .label("Open listing")
                            .disabled(url.is_none())
                            .on_click(move |_, _, cx| {
                                if let Some(url) = &url {
                                    cx.open_url(url);
                                }
                            }),
                    )
                    .child(
                        Button::new("reveal-photos")
                            .small()
                            .icon(IconName::FolderOpen)
                            .label("Photos")
                            .disabled(photo_paths.is_empty())
                            .on_click(move |_, _, cx| cx.reveal_path(&photos_dir)),
                    )
                    .child(div().flex_1())
                    .child(
                        Button::new("delete-home")
                            .small()
                            .ghost()
                            .icon(IconName::Delete)
                            .tooltip("Delete from Home Safe")
                            .on_click(cx.listener(|this, _, window, cx| {
                                if let Some(home) = this.selected.as_ref().map(|s| s.home.clone()) {
                                    this.delete_home(home, window, cx);
                                }
                            })),
                    ),
            )
            .child(facts)
            .when(photo_paths.len() > 1 || pending > 0, |this| {
                this.child(
                    v_flex()
                        .gap_2()
                        .child(div().text_xs().text_color(theme.muted_foreground).child(
                            if pending > 0 {
                                format!("Photos ({} of {} downloaded)", photo_paths.len(), selected.images.len())
                            } else {
                                format!("Photos ({})", photo_paths.len())
                            },
                        ))
                        .child(h_flex().flex_wrap().gap_1().children(
                            photo_paths.iter().skip(1).map(|p| {
                                img(p.clone())
                                    .w(px(112.))
                                    .h(px(76.))
                                    .rounded(px(4.))
                                    .object_fit(ObjectFit::Cover)
                            }),
                        )),
                )
            })
            .when_some(home.description.clone(), |this, description| {
                this.child(div().text_sm().line_height(rems(1.4)).child(description))
            })
    }

    fn render_cards(&self, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let table = self.table.read(cx).delegate();
        let selected_id = self.selected.as_ref().map(|s| s.home.id);
        let homes: Vec<Home> = table.visible().cloned().collect();
        if homes.is_empty() {
            return h_flex()
                .size_full()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child(table.empty_message())
                .into_any_element();
        }

        let mut width = window.viewport_size().width - px(2. * homes_cards::PADDING);
        if self.selected.is_some() {
            width -= px(DETAIL_WIDTH);
        }
        let cards = homes.into_iter().enumerate().map(|(row, home)| {
            let selected = Some(home.id) == selected_id;
            let url = home.url.clone();
            let on_delete = cx.weak_entity();
            homes_cards::card(row, &home, selected, cx)
                .min_w_0()
                .on_click(cx.listener(move |this, event: &ClickEvent, _, cx| {
                    if event.click_count() >= 2 {
                        if let Some(url) = &url {
                            cx.open_url(url);
                        }
                        return;
                    }
                    this.table.update(cx, |t, cx| t.set_selected_row(row, cx));
                }))
                .context_menu(move |menu, _, _| {
                    let on_delete = on_delete.clone();
                    let home = home.clone();
                    menu.item(PopupMenuItem::new("Delete…").icon(IconName::Delete).on_click(
                        move |_, window, cx| {
                            on_delete
                                .update(cx, |this, cx| this.delete_home(home.clone(), window, cx))
                                .ok();
                        },
                    ))
                })
        });

        homes_cards::grid(homes_cards::columns(width), cards).into_any_element()
    }
}

impl Render for HomesView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let detail = self.selected.as_ref().map(|s| self.render_detail(s, cx));
        let content = match self.mode {
            ViewMode::Cards => self.render_cards(window, cx),
            ViewMode::Table => DataTable::new(&self.table).stripe(true).into_any_element(),
        };
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(self.render_toolbar(cx))
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
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .min_w_0()
                            .child(content),
                    )
                    .children(detail),
            )
    }
}
