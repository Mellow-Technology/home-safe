//! The page for one home: photos, facts and description.

use std::path::{Path, PathBuf};

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use home_safe_core::Home;

use super::{DETAIL_CONTEXT, Detail, GoBack, HomesView};
use crate::format;

const MAX_CONTENT_WIDTH: f32 = 1120.;
const PAGE_PADDING: f32 = 32.;
const HERO_HEIGHT: f32 = 440.;
const GAP: f32 = 8.;
const THUMB: (f32, f32) = (168., 112.);

impl HomesView {
    pub(super) fn render_detail(
        &self,
        detail: &Detail,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let home = &detail.home;
        let photos: Vec<PathBuf> = detail.photos().into_iter().map(|(p, _)| p).collect();
        let pending = detail.images.len() - photos.len();
        let width = (f32::from(window.viewport_size().width) - 2. * PAGE_PADDING).clamp(320., MAX_CONTENT_WIDTH);

        v_flex()
            .size_full()
            .track_focus(&self.detail_focus)
            .key_context(DETAIL_CONTEXT)
            .on_action(cx.listener(|this, _: &GoBack, window, cx| this.go_back(window, cx)))
            .child(self.render_detail_header(home, !photos.is_empty(), cx))
            .child(
                div().id("detail-scroll").flex_1().min_h_0().overflow_y_scroll().child(
                    v_flex()
                        .w(px(width))
                        .mx_auto()
                        .py_6()
                        .gap_6()
                        .child(self.render_hero(&photos, pending, width, cx))
                        .child(render_summary(home, cx))
                        .child(render_facts(home, cx))
                        .when(photos.len() > 1 || pending > 0, |this| {
                            this.child(self.render_photo_grid(&photos, detail.images.len(), cx))
                        })
                        .when_some(home.description.clone(), |this, description| {
                            this.child(
                                v_flex()
                                    .gap_2()
                                    .child(section_title("About this home"))
                                    .child(div().text_sm().line_height(rems(1.5)).child(description)),
                            )
                        }),
                ),
            )
    }

    fn render_detail_header(&self, home: &Home, has_photos: bool, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let url = home.url.clone();
        let photos_dir = self.backend.images_dir.join(home.id.to_string());
        let theme = cx.theme();
        h_flex()
            .gap_3()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(theme.border)
            .child(
                Button::new("back")
                    .small()
                    .ghost()
                    .icon(IconName::ArrowLeft)
                    .label("All homes")
                    .tooltip("Back to all homes (Esc)")
                    .on_click(cx.listener(|this, _, window, cx| this.go_back(window, cx))),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(home.full_address()),
            )
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
                    .label("Photos folder")
                    .disabled(!has_photos)
                    .on_click(move |_, _, cx| cx.reveal_path(&photos_dir)),
            )
            .child(
                Button::new("delete-home")
                    .small()
                    .ghost()
                    .icon(gpui_kit::assets::IconName::Trash)
                    .tooltip("Delete from Home Safe")
                    .on_click(cx.listener(|this, _, window, cx| {
                        if let Some(home) = this.detail.as_ref().map(|d| d.home.clone()) {
                            this.delete_home(home, window, cx);
                        }
                    })),
            )
    }

    /// One large photo, plus two smaller ones beside it when there are enough.
    fn render_hero(&self, photos: &[PathBuf], pending: usize, width: f32, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        if photos.is_empty() {
            return v_flex()
                .w(px(width))
                .h(px(HERO_HEIGHT * 0.6))
                .rounded(px(8.))
                .bg(theme.muted)
                .items_center()
                .justify_center()
                .gap_2()
                .text_color(theme.muted_foreground)
                .child(Icon::new(gpui_kit::assets::IconName::House).size_10())
                .child(if pending > 0 { "Downloading photos…" } else { "No photos" })
                .into_any_element();
        }

        let mosaic = photos.len() >= 3;
        let main_w = if mosaic { ((width - GAP) * 2. / 3.).floor() } else { width };
        let side_w = width - main_w - GAP;
        let side_h = (HERO_HEIGHT - GAP) / 2.;

        div()
            .relative()
            .child(
                h_flex()
                    .gap(px(GAP))
                    .child(self.photo_tile(0, &photos[0], main_w, HERO_HEIGHT, cx))
                    .when(mosaic, |this| {
                        this.child(
                            v_flex()
                                .gap(px(GAP))
                                .child(self.photo_tile(1, &photos[1], side_w, side_h, cx))
                                .child(self.photo_tile(2, &photos[2], side_w, side_h, cx)),
                        )
                    }),
            )
            .when(photos.len() > 1, |this| {
                this.child(
                    div().absolute().bottom_3().right_3().child(
                        Button::new("view-all-photos")
                            .small()
                            .icon(gpui_kit::assets::IconName::Images)
                            .label(format!("View all {} photos", photos.len()))
                            .on_click(cx.listener(|this, _, window, cx| this.open_photo(0, window, cx))),
                    ),
                )
            })
            .into_any_element()
    }

    fn render_photo_grid(&self, photos: &[PathBuf], total: usize, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let title = if photos.len() < total {
            format!("Photos ({} of {total} downloaded)", photos.len())
        } else {
            format!("Photos ({total})")
        };
        v_flex().gap_2().child(section_title(title)).child(
            h_flex().flex_wrap().gap(px(GAP)).children(
                photos
                    .iter()
                    .enumerate()
                    .map(|(ix, path)| self.photo_tile(ix, path, THUMB.0, THUMB.1, cx)),
            ),
        )
    }

    /// A clickable photo that opens the viewer at `ix`. Sizes are explicit:
    /// with relative ones GPUI sizes images from their aspect ratio.
    fn photo_tile(&self, ix: usize, path: &Path, w: f32, h: f32, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        div()
            .id(SharedString::from(format!("photo-{ix}-{w}")))
            .w(px(w))
            .h(px(h))
            .flex_shrink_0()
            .rounded(px(6.))
            .overflow_hidden()
            .cursor_pointer()
            .hover(|this| this.opacity(0.9))
            .on_click(cx.listener(move |this, _, window, cx| this.open_photo(ix, window, cx)))
            .child(
                img(path.to_path_buf())
                    .w(px(w))
                    .h(px(h))
                    .rounded(px(6.))
                    .object_fit(ObjectFit::Cover),
            )
    }
}

fn render_summary(home: &Home, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let status = format::humanize(home.status.as_deref());
    let ppsf = format::money(home.price_per_sqft());
    let price = format::money(home.price);
    let specs = [
        home.beds.map(|b| format!("{} bd", format::decimal(Some(b)))),
        home.baths.map(|b| format!("{} ba", format::decimal(Some(b)))),
        home.sqft.map(|s| format!("{} sq ft", format::number(Some(s)))),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");

    v_flex()
        .gap_1()
        .child(
            h_flex()
                .items_end()
                .gap_3()
                .child(
                    div()
                        .text_3xl()
                        .font_weight(FontWeight::BOLD)
                        .child(if price.is_empty() { "Price unknown".to_string() } else { price }),
                )
                .when(!ppsf.is_empty(), |this| {
                    this.child(div().pb_1().text_color(theme.muted_foreground).child(format!("{ppsf}/sq ft")))
                })
                .child(div().flex_1())
                .when(!status.is_empty(), |this| this.child(Tag::secondary().rounded_full().child(status))),
        )
        .when(!specs.is_empty(), |this| this.child(div().text_lg().child(specs)))
        .child(div().text_color(theme.muted_foreground).child(home.full_address()))
}

fn render_facts(home: &Home, cx: &App) -> impl IntoElement + use<> {
    let theme = cx.theme();
    let facts = [
        ("Home type", format::humanize(home.home_type.as_deref())),
        ("Year built", home.year_built.map(|y| y.to_string()).unwrap_or_default()),
        ("Lot size", home.lot_sqft.map(|s| format!("{} sq ft", format::number(Some(s)))).unwrap_or_default()),
        ("HOA", home.hoa_monthly.map(|h| format!("{}/mo", format::money(Some(h)))).unwrap_or_default()),
        ("Zestimate", format::money(home.zestimate)),
        ("Rent Zestimate", home.rent_zestimate.map(|r| format!("{}/mo", format::money(Some(r)))).unwrap_or_default()),
        ("Days on Zillow", format::number(home.days_on_market)),
        ("Saved", format::date(&home.created_at)),
    ];
    v_flex().gap_2().child(section_title("Facts")).child(
        div()
            .grid()
            .grid_cols(4)
            .gap_4()
            .p_4()
            .rounded(px(8.))
            .border_1()
            .border_color(theme.border)
            .children(facts.into_iter().map(|(label, value)| {
                v_flex()
                    .min_w_0()
                    .gap_0p5()
                    .child(div().text_xs().text_color(theme.muted_foreground).child(label))
                    .child(div().truncate().child(if value.is_empty() { "—".to_string() } else { value }))
            })),
    )
}

fn section_title(title: impl Into<SharedString>) -> impl IntoElement {
    div().text_lg().font_weight(FontWeight::SEMIBOLD).child(title.into())
}
