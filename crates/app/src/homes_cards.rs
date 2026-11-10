//! Card view: one card per home with its photo and headline facts.

use std::path::PathBuf;

use gpui_kit::component::tag::Tag;
use gpui_kit::component::{ActiveTheme as _, Icon, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use home_safe_core::Home;

use crate::format;

pub const CARD_MIN_WIDTH: f32 = 280.;
pub const GAP: f32 = 16.;
pub const PADDING: f32 = 16.;
const PHOTO_HEIGHT: f32 = 180.;
const RADIUS: f32 = 8.;

/// Number of grid columns that fit `width` with cards at least `CARD_MIN_WIDTH` wide.
pub fn columns(width: Pixels) -> u16 {
    let width = f32::from(width);
    (((width + GAP) / (CARD_MIN_WIDTH + GAP)).floor() as u16).max(1)
}

/// Scrollable grid of cards filling its parent.
pub fn grid(columns: u16, cards: impl IntoIterator<Item = impl IntoElement>) -> Stateful<Div> {
    div().id("cards").size_full().overflow_y_scroll().p(px(PADDING)).child(
        div()
            .grid()
            .grid_cols(columns)
            .gap(px(GAP))
            .children(cards),
    )
}

/// `3 bd · 2 ba · 1,850 sq ft`, skipping unknown values.
fn summary(home: &Home) -> String {
    [
        home.beds.map(|b| format!("{} bd", format::decimal(Some(b)))),
        home.baths.map(|b| format!("{} ba", format::decimal(Some(b)))),
        home.sqft.map(|s| format!("{} sq ft", format::number(Some(s)))),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

pub fn card(ix: usize, home: &Home, selected: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let muted = theme.muted_foreground;

    let photo = match &home.cover_image {
        // Explicit height: with a relative one, GPUI sizes the image from the
        // photo's aspect ratio and it draws taller than its box.
        Some(path) => img(PathBuf::from(path))
            .w_full()
            .h(px(PHOTO_HEIGHT))
            .rounded_t(px(RADIUS - 1.))
            .object_fit(ObjectFit::Cover)
            .into_any_element(),
        None => div()
            .size_full()
            .rounded_t(px(RADIUS - 1.))
            .bg(theme.muted)
            .flex()
            .items_center()
            .justify_center()
            .text_color(muted)
            .child(Icon::new(gpui_kit::assets::IconName::House).size_8())
            .into_any_element(),
    };

    let status = format::humanize(home.status.as_deref());
    let address = home.address.clone().unwrap_or_else(|| format!("zpid {}", home.source_id));
    let locality = [home.city.as_deref(), home.state.as_deref(), home.zip.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" ");
    let price = format::money(home.price);
    let ppsf = format::money(home.price_per_sqft());

    // No overflow clipping on the card: a clipped grid item has a zero
    // minimum height, which let rows shrink and the text slide over the photo.
    v_flex()
        .id(("home-card", ix))
        .rounded(px(RADIUS))
        .bg(theme.background)
        .border_1()
        .border_color(if selected { theme.primary } else { theme.border })
        .when(selected, |this| this.shadow_md())
        .hover(|this| this.shadow_md())
        .cursor_pointer()
        .child(
            div()
                .w_full()
                .h(px(PHOTO_HEIGHT))
                .flex_shrink_0()
                .rounded_t(px(RADIUS - 1.))
                .overflow_hidden()
                .child(photo),
        )
        .child(
            v_flex()
                .flex_shrink_0()
                .p_3()
                .gap_1()
                .child(
                    h_flex()
                        .items_end()
                        .gap_2()
                        .child(
                            div()
                                .text_xl()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(if price.is_empty() { "—".to_string() } else { price }),
                        )
                        .when(!ppsf.is_empty(), |this| {
                            this.child(div().text_xs().text_color(muted).child(format!("{ppsf}/sq ft")))
                        }),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .child(div().flex_1().min_w_0().truncate().text_sm().child(summary(home)))
                        .when(!status.is_empty(), |this| {
                            this.child(Tag::secondary().small().rounded_full().child(status))
                        }),
                )
                .child(div().text_sm().truncate().child(address))
                .child(div().text_xs().text_color(muted).truncate().child(locality))
                .child(
                    h_flex()
                        .pt_1()
                        .text_xs()
                        .text_color(muted)
                        .child(div().flex_1().truncate().child(format::humanize(home.home_type.as_deref())))
                        .child(format!("Saved {}", format::date(&home.created_at))),
                ),
        )
}

