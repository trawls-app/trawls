use gpui::{
    App, Context, FocusHandle, Focusable, InteractiveElement as _, IntoElement, KeyDownEvent,
    ParentElement as _, Render, StatefulInteractiveElement as _, Styled as _, Window, div, img, px,
    relative,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _, TitleBar,
    button::{Button, ButtonVariants as _},
    h_flex,
    sidebar::{Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    v_flex,
};

use crate::ui::app::Trawls;

impl Trawls {
    fn render_active_step(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        match self.active_step {
            0 => self.render_image_step(true, cx).into_any_element(),
            1 => self.render_image_step(false, cx).into_any_element(),
            2 => self.render_processing_step(cx).into_any_element(),
            _ => self.render_preview_step(cx).into_any_element(),
        }
    }

    fn render_titlebar(&self) -> impl IntoElement {
        TitleBar::new().child(
            h_flex()
                .w_full()
                .items_center()
                .justify_between()
                .px_2()
                .child(
                    v_flex()
                        .gap_0()
                        .child(div().font_semibold().child("Trawls")),
                )
                .child(div()),
        )
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Sidebar::left()
            .header(
                SidebarHeader::new()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().sidebar_accent)
                            .size_8()
                            .flex_shrink_0()
                            .child(img("examples/trawls-logo.png").w(px(22.0)).h(px(22.0))),
                    )
                    .child(
                        v_flex()
                            .gap_0()
                            .text_sm()
                            .flex_1()
                            .line_height(relative(1.25))
                            .overflow_hidden()
                            .text_ellipsis()
                            .child("Startrail merger")
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("From RAW to a nightsky DNG"),
                            ),
                    ),
            )
            .child(
                SidebarGroup::new("Steps").child(
                    SidebarMenu::new()
                        .child(
                            SidebarMenuItem::new("Lightframes")
                                .icon(IconName::Star)
                                .active(self.active_step == 0)
                                .suffix(format!("{}", self.lightframes.len()))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_active_step(0, cx);
                                })),
                        )
                        .child(
                            SidebarMenuItem::new("Darkframes")
                                .icon(IconName::Moon)
                                .active(self.active_step == 1)
                                .suffix(format!("{}", self.darkframes.len()))
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_active_step(1, cx);
                                })),
                        )
                        .child({
                            let item = SidebarMenuItem::new("Processing")
                                .icon(IconName::Settings2)
                                .active(self.active_step == 2)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_active_step(2, cx);
                                }));
                            if self.processing_running {
                                item.suffix(
                                    Icon::new(IconName::LoaderCircle)
                                        .xsmall()
                                        .text_color(cx.theme().muted_foreground),
                                )
                            } else if self.preview.is_some() {
                                item.suffix(
                                    Icon::new(IconName::CircleCheck)
                                        .xsmall()
                                        .text_color(cx.theme().success),
                                )
                            } else {
                                item
                            }
                        })
                        .child({
                            let item = SidebarMenuItem::new("Preview")
                                .icon(IconName::Eye)
                                .active(self.active_step == 3)
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.set_active_step(3, cx);
                                }));
                            if self.preview.is_some() {
                                item.suffix(
                                    Icon::new(IconName::CircleCheck)
                                        .xsmall()
                                        .text_color(cx.theme().success),
                                )
                            } else {
                                item
                            }
                        }),
                ),
            )
            .footer(
                SidebarFooter::new().child(
                    Button::new("sidebar-version-link")
                        .link()
                        .label("trawls v0.5.0")
                        .on_click(|_, _, cx| cx.open_url("https://github.com/trawls-app/trawls")),
                ),
            )
    }

    fn render_pinned_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .id("header")
            .justify_between()
            .items_center()
            .p_4()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .gap_1()
                    .child(div().text_xl().child(self.step_title()))
                    .child(
                        div()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.step_description()),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("prev-step")
                            .small()
                            .icon(IconName::ChevronLeft)
                            .label("Previous")
                            .disabled(self.active_step == 0)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_active_step(this.active_step.saturating_sub(1), cx);
                            })),
                    )
                    .child({
                        let next_disabled = self.active_step >= 3
                            || (self.active_step == 0 && self.lightframes.is_empty());
                        Button::new("next-step")
                            .small()
                            .primary()
                            .icon(IconName::ChevronRight)
                            .label("Next")
                            .disabled(next_disabled)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.set_active_step((this.active_step + 1).min(3), cx);
                            }))
                    }),
            )
    }
}

impl Focusable for Trawls {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Trawls {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = self.render_active_step(cx);

        div()
            .id("trawls-app")
            .size_full()
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key.as_str() == "escape" {
                    if this.frame_preview.is_some() || this.frame_preview_loading {
                        this.close_frame_preview(cx);
                    }
                }
            }))
            .bg(cx.theme().background)
            .child(
                v_flex().size_full().child(self.render_titlebar()).child(
                    h_flex()
                        .flex_1()
                        .h_full()
                        .items_start()
                        .overflow_hidden()
                        .child(self.render_sidebar(cx))
                        .child(
                            v_flex()
                                .flex_1()
                                .h_full()
                                .overflow_hidden()
                                .gap_0()
                                .child(self.render_pinned_header(cx))
                                .child(
                                    div()
                                        .id("content-scroll")
                                        .flex_1()
                                        .overflow_y_scroll()
                                        .p_3()
                                        .child(content),
                                ),
                        ),
                ),
            )
    }
}
