use gpui::{
    AnyElement, Context, InteractiveElement as _, IntoElement, Keystroke, ParentElement as _,
    Pixels, Point, ScrollDelta, ScrollHandle, ScrollWheelEvent, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, div, point, prelude::FluentBuilder as _,
    px,
};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::Button,
    h_flex,
    kbd::Kbd,
    scroll::{Scrollbar, ScrollbarShow, ScrollbarState},
    v_flex,
};

use crate::ui::app::Trawls;

const MIN_ZOOM: f32 = 0.02;
const MAX_ZOOM: f32 = 8.0;

pub struct ImageViewerState {
    pub zoom: f32,
    pub scroll_handle: ScrollHandle,
    pub scroll_state: ScrollbarState,
}

impl Default for ImageViewerState {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            scroll_handle: ScrollHandle::new(),
            scroll_state: ScrollbarState::default(),
        }
    }
}

impl ImageViewerState {
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    }

    pub fn zoom_in(&mut self) {
        self.set_zoom(self.zoom * 1.25);
    }

    pub fn zoom_out(&mut self) {
        self.set_zoom(self.zoom / 1.25);
    }

    pub fn fit_to_viewport(&mut self, viewport: gpui::Size<Pixels>, image_w: u32, image_h: u32) {
        let available_w = (f32::from(viewport.width) - 360.0).max(220.0);
        let available_h = (f32::from(viewport.height) - 250.0).max(180.0);
        let zoom_w = available_w / image_w as f32;
        let zoom_h = available_h / image_h as f32;
        self.set_zoom(zoom_w.min(zoom_h));
    }

    pub fn zoom_at_cursor(
        &mut self,
        cursor: Point<Pixels>,
        zoom_in: bool,
        window: &mut Window,
        cx: &mut gpui::App,
    ) {
        let old_zoom = self.zoom;
        let new_zoom = if zoom_in {
            (old_zoom * 1.25).clamp(MIN_ZOOM, MAX_ZOOM)
        } else {
            (old_zoom / 1.25).clamp(MIN_ZOOM, MAX_ZOOM)
        };
        if (new_zoom - old_zoom).abs() < f32::EPSILON {
            return;
        }

        let viewport_bounds = self.scroll_handle.bounds();
        let old_offset = self.scroll_handle.offset();

        let local_x = (f32::from(cursor.x) - f32::from(viewport_bounds.origin.x))
            .clamp(0.0, f32::from(viewport_bounds.size.width));
        let local_y = (f32::from(cursor.y) - f32::from(viewport_bounds.origin.y))
            .clamp(0.0, f32::from(viewport_bounds.size.height));

        let offset_x = f32::from(old_offset.x);
        let offset_y = f32::from(old_offset.y);
        let scale = new_zoom / old_zoom.max(0.0001);

        let target_offset = point(
            px(offset_x * scale + local_x * (1.0 - scale)),
            px(offset_y * scale + local_y * (1.0 - scale)),
        );

        self.zoom = new_zoom;
        self.scroll_handle.set_offset(target_offset);
        let handle = self.scroll_handle.clone();
        window.defer(cx, move |_, _| {
            handle.set_offset(target_offset);
        });
    }
}

pub struct ImageViewerParams<'a> {
    pub id_prefix: &'a str,
    pub image_source: gpui::AnyElement,
    pub image_width: u32,
    pub image_height: u32,
    pub info_bar: Option<gpui::AnyElement>,
    pub left_header: Option<gpui::AnyElement>,
}

pub fn render_image_viewer(
    params: ImageViewerParams<'_>,
    state: &ImageViewerState,
    state_accessor_mut: impl Fn(&mut Trawls) -> &mut ImageViewerState + 'static + Copy,
    cx: &mut Context<Trawls>,
) -> AnyElement {
    let zoom = state.zoom;
    let zoomed_w = params.image_width as f32 * zoom;
    let zoomed_h = params.image_height as f32 * zoom;

    let panel_id: SharedString = format!("{}-panel", params.id_prefix).into();
    let scroll_id: SharedString = format!("{}-scroll", params.id_prefix).into();
    let zoom_out_id: SharedString = format!("{}-zoom-out", params.id_prefix).into();
    let zoom_fit_id: SharedString = format!("{}-zoom-fit", params.id_prefix).into();
    let zoom_100_id: SharedString = format!("{}-zoom-100", params.id_prefix).into();
    let zoom_in_id: SharedString = format!("{}-zoom-in", params.id_prefix).into();

    let image_w = params.image_width;
    let image_h = params.image_height;

    v_flex()
        .w_full()
        .h_full()
        .gap_3()
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .gap_2()
                .child(
                    params
                        .left_header
                        .unwrap_or_else(|| div().into_any_element()),
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            Button::new(zoom_out_id)
                                .small()
                                .icon(IconName::Minus)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    state_accessor_mut(this).zoom_out();
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new(zoom_fit_id)
                                .small()
                                .label("Fit")
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    let vp = window.viewport_size();
                                    state_accessor_mut(this).fit_to_viewport(vp, image_w, image_h);
                                    cx.notify();
                                })),
                        )
                        .child(Button::new(zoom_100_id).small().label("100%").on_click(
                            cx.listener(move |this, _, _, cx| {
                                state_accessor_mut(this).set_zoom(1.0);
                                cx.notify();
                            }),
                        ))
                        .child(
                            Button::new(zoom_in_id)
                                .small()
                                .icon(IconName::Plus)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    state_accessor_mut(this).zoom_in();
                                    cx.notify();
                                })),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(Kbd::new(
                                    Keystroke::parse(if cfg!(target_os = "macos") {
                                        "cmd"
                                    } else {
                                        "ctrl"
                                    })
                                    .expect("valid keystroke"),
                                ))
                                .child("+scroll"),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{:.0}%", zoom * 100.0)),
                        ),
                ),
        )
        .child(
            v_flex()
                .id(panel_id.clone())
                .relative()
                .flex_1()
                .min_h_0()
                .w_full()
                .on_scroll_wheel(
                    cx.listener(move |this, event: &ScrollWheelEvent, window, cx| {
                        if !event.modifiers.secondary() {
                            return;
                        }
                        let delta_y = match event.delta {
                            ScrollDelta::Pixels(delta) => f32::from(delta.y),
                            ScrollDelta::Lines(delta) => delta.y * 16.0,
                        };
                        if delta_y != 0.0 {
                            state_accessor_mut(this).zoom_at_cursor(
                                event.position,
                                delta_y < 0.0,
                                window,
                                cx,
                            );
                            cx.notify();
                        }
                        cx.stop_propagation();
                    }),
                )
                .child(
                    div()
                        .id(scroll_id.clone())
                        .track_scroll(&state.scroll_handle)
                        .overflow_x_scroll()
                        .overflow_y_scroll()
                        .size_full()
                        .child(
                            div()
                                .w(px(zoomed_w))
                                .h(px(zoomed_h))
                                .flex_shrink_0()
                                .child(params.image_source),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .right_0()
                        .bottom_0()
                        .child(
                            Scrollbar::both(&state.scroll_state, &state.scroll_handle)
                                .scrollbar_show(ScrollbarShow::Always),
                        ),
                ),
        )
        .when_some(params.info_bar, |el, info| el.child(info))
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Size, px};

    #[test]
    fn default_zoom() {
        let state = ImageViewerState::default();
        assert!((state.zoom - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn zoom_in() {
        let mut state = ImageViewerState::default();
        state.zoom_in();
        assert!((state.zoom - 1.25).abs() < 0.01);
    }

    #[test]
    fn zoom_out() {
        let mut state = ImageViewerState::default();
        state.zoom_out();
        assert!((state.zoom - 0.8).abs() < 0.01);
    }

    #[test]
    fn zoom_clamped_min() {
        let mut state = ImageViewerState::default();
        state.set_zoom(0.001);
        assert!((state.zoom - MIN_ZOOM).abs() < f32::EPSILON);
    }

    #[test]
    fn zoom_clamped_max() {
        let mut state = ImageViewerState::default();
        state.set_zoom(100.0);
        assert!((state.zoom - MAX_ZOOM).abs() < f32::EPSILON);
    }

    #[test]
    fn fit_to_viewport_landscape() {
        let mut state = ImageViewerState::default();
        let viewport = Size {
            width: px(1920.0),
            height: px(1080.0),
        };
        state.fit_to_viewport(viewport, 6000, 4000);
        assert!((state.zoom - 0.2075).abs() < 0.01);
    }

    #[test]
    fn fit_to_viewport_small_image() {
        let mut state = ImageViewerState::default();
        let viewport = Size {
            width: px(1920.0),
            height: px(1080.0),
        };
        state.fit_to_viewport(viewport, 100, 100);
        assert!((state.zoom - MAX_ZOOM).abs() < f32::EPSILON);
    }
}
