//! Full-window photo viewer over the home page. Left/right arrows step
//! through the photos (wrapping at either end); Escape closes it.

use gpui_kit::component::{Icon, IconName, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use super::{ClosePhoto, Detail, HomesView, NextPhoto, PrevPhoto, VIEWER_CONTEXT};

const BAR_HEIGHT: f32 = 56.;
const SIDE_WIDTH: f32 = 80.;

impl HomesView {
    pub(super) fn render_viewer(
        &self,
        detail: &Detail,
        ix: usize,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let photos = detail.photos();
        let count = photos.len();
        let Some((path, image)) = photos.get(ix) else {
            return div().into_any_element();
        };

        // Fit the photo in the space between the bars and arrow buttons.
        let viewport = window.viewport_size();
        let max_w = (f32::from(viewport.width) - 2. * SIDE_WIDTH).max(100.);
        let max_h = (f32::from(viewport.height) - 2. * BAR_HEIGHT).max(100.);
        let (w, h) = match (image.width, image.height) {
            (Some(w), Some(h)) if w > 0 && h > 0 => {
                let scale = (max_w / w as f32).min(max_h / h as f32);
                (w as f32 * scale, h as f32 * scale)
            }
            _ => (max_w, max_h),
        };

        let white = gpui_kit::white();
        let muted = white.opacity(0.7);
        let caption = image.caption.clone().filter(|c| !c.trim().is_empty());

        v_flex()
            .id("photo-viewer")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .occlude()
            .bg(gpui_kit::black().opacity(0.92))
            .text_color(white)
            .track_focus(&self.viewer_focus)
            .key_context(VIEWER_CONTEXT)
            .on_action(cx.listener(|this, _: &PrevPhoto, _, cx| this.step_photo(-1, cx)))
            .on_action(cx.listener(|this, _: &NextPhoto, _, cx| this.step_photo(1, cx)))
            .on_action(cx.listener(|this, _: &ClosePhoto, window, cx| this.close_photo(window, cx)))
            // Clicking the backdrop (anything not stopped below) closes the viewer.
            .on_click(cx.listener(|this, _, window, cx| this.close_photo(window, cx)))
            .child(
                h_flex()
                    .h(px(BAR_HEIGHT))
                    .flex_shrink_0()
                    .px_4()
                    .child(div().w(px(SIDE_WIDTH)))
                    .child(
                        div()
                            .flex_1()
                            .text_center()
                            .text_sm()
                            .text_color(muted)
                            .child(format!("{} / {count}", ix + 1)),
                    )
                    .child(
                        h_flex().w(px(SIDE_WIDTH)).justify_end().child(stop_click(
                            round_button("viewer-close", IconName::Close, 36.)
                                .on_click(cx.listener(|this, _, window, cx| this.close_photo(window, cx))),
                        )),
                    ),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_h_0()
                    .items_center()
                    .child(
                        h_flex()
                            .w(px(SIDE_WIDTH))
                            .justify_center()
                            .when(count > 1, |this| {
                                this.child(stop_click(
                                    round_button("viewer-prev", IconName::ChevronLeft, 48.)
                                        .on_click(cx.listener(|this, _, _, cx| this.step_photo(-1, cx))),
                                ))
                            }),
                    )
                    .child(
                        h_flex().flex_1().justify_center().child(stop_click(
                            img(path.clone())
                                .id(("viewer-photo", ix))
                                .w(px(w))
                                .h(px(h))
                                .object_fit(ObjectFit::Contain),
                        )),
                    )
                    .child(
                        h_flex()
                            .w(px(SIDE_WIDTH))
                            .justify_center()
                            .when(count > 1, |this| {
                                this.child(stop_click(
                                    round_button("viewer-next", IconName::ChevronRight, 48.)
                                        .on_click(cx.listener(|this, _, _, cx| this.step_photo(1, cx))),
                                ))
                            }),
                    ),
            )
            .child(
                h_flex()
                    .h(px(BAR_HEIGHT))
                    .flex_shrink_0()
                    .justify_center()
                    .px(px(SIDE_WIDTH))
                    .text_sm()
                    .text_color(muted)
                    .when_some(caption, |this, caption| this.child(div().truncate().child(caption))),
            )
            .into_any_element()
    }
}

/// A white icon in a translucent circle, visible on the dark backdrop.
/// (Theme buttons draw dark icons here, which disappear on black.)
fn round_button(id: &'static str, icon: IconName, size: f32) -> Stateful<Div> {
    let white = gpui_kit::white();
    div()
        .id(id)
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .cursor_pointer()
        .bg(white.opacity(0.14))
        .hover(|this| this.bg(white.opacity(0.28)))
        .active(|this| this.bg(white.opacity(0.4)))
        .child(Icon::new(icon).size(px(size * 0.55)).text_color(white))
}

/// Wraps `child` so clicks on it don't reach the backdrop's close handler.
fn stop_click(child: impl IntoElement) -> impl IntoElement {
    div()
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_up(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(child)
}
