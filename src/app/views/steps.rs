use gpui::{
    Context, IntoElement, ParentElement as _, Styled as _, div, img, prelude::FluentBuilder as _,
    px,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    progress::Progress,
    tab::{Tab, TabBar},
    v_flex,
};

use crate::{
    app::TrawlsDummy,
    model::MergeMode,
    samples::{sample_darkframes, sample_lightframes},
};

impl TrawlsDummy {
    pub(crate) fn render_image_step(
        &self,
        is_light: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let rows = if is_light {
            Self::sorted_frames(&self.lightframes, self.light_sort)
        } else {
            Self::sorted_frames(&self.darkframes, self.dark_sort)
        };
        let count = rows.len();

        v_flex()
            .gap_4()
            .when(Self::error_warning(&rows), |this| {
                this.child(
                    Alert::warning(
                        if is_light {
                            "warn-light-error"
                        } else {
                            "warn-dark-error"
                        },
                        "The metadata of some files could not be loaded.",
                    )
                    .banner(),
                )
            })
            .when(Self::camera_setting_warning(&rows), |this| {
                this.child(
                    Alert::warning(
                        if is_light {
                            "warn-light-settings"
                        } else {
                            "warn-dark-settings"
                        },
                        "The camera settings between some frames changed.",
                    )
                    .banner(),
                )
            })
            .when(is_light && Self::interval_warning(&rows), |this| {
                this.child(
                    Alert::warning(
                        "warn-light-interval",
                        "Some frame intervals are much larger than the average.",
                    )
                    .banner(),
                )
            })
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_2()
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().sidebar.alpha(0.45))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new(if is_light {
                                    "pick-lights"
                                } else {
                                    "pick-darks"
                                })
                                .success()
                                .icon(IconName::Search)
                                .label("Select images")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if is_light {
                                            this.lightframes = sample_lightframes();
                                        } else {
                                            this.darkframes = sample_darkframes();
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                Button::new(if is_light {
                                    "clear-lights"
                                } else {
                                    "clear-darks"
                                })
                                .warning()
                                .icon(IconName::Minus)
                                .label("Clear list")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if is_light {
                                            this.lightframes.clear();
                                        } else {
                                            this.darkframes.clear();
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(
                        h_flex().gap_2().items_center().child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("Frames: {count}")),
                        ),
                    ),
            )
            .child(self.render_frame_table(&rows, is_light, is_light, cx))
    }

    pub(crate) fn render_processing_step(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mode_ix = match self.merge_mode {
            MergeMode::Normal => 0,
            MergeMode::Falling => 1,
            MergeMode::Raising => 2,
        };

        v_flex()
            .gap_4()
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().sidebar.alpha(0.45))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::BookOpen))
                            .child(div().font_semibold().child("Output path")),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Input::new(&self.output_input).disabled(true).w_full())
                            .child(
                                Button::new("choose-output")
                                    .primary()
                                    .icon(IconName::Search)
                                    .label("Choose output")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.choose_dummy_output(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("clear-output")
                                    .icon(IconName::Minus)
                                    .label("Clear")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.clear_output(window, cx);
                                    })),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(Icon::new(IconName::Info).xsmall())
                            .child("Where the resulting DNG file should be saved."),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().sidebar.alpha(0.45))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::Settings2))
                            .child(div().font_semibold().child("Mode selection")),
                    )
                    .child(
                        TabBar::new("merge-mode")
                            .segmented()
                            .w_full()
                            .selected_index(mode_ix)
                            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                                this.merge_mode = match ix {
                                    1 => MergeMode::Falling,
                                    2 => MergeMode::Raising,
                                    _ => MergeMode::Normal,
                                };
                                cx.notify();
                            }))
                            .child(Tab::new().label("Normal"))
                            .child(Tab::new().label("Falling Comets"))
                            .child(Tab::new().label("Raising Comets")),
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .w_full()
                                    .child(img("examples/mode_normal.jpg").h(px(130.)))
                                    .child(
                                        div()
                                            .text_xs()
                                            .child("Normal: all images weighted equally."),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .w_full()
                                    .child(img("examples/mode_falling.jpg").h(px(130.)))
                                    .child(
                                        div()
                                            .text_xs()
                                            .child("Falling: later images become darker."),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .w_full()
                                    .child(img("examples/mode_raising.jpg").h(px(130.)))
                                    .child(
                                        div()
                                            .text_xs()
                                            .child("Raising: earlier images become darker."),
                                    ),
                            ),
                    ),
            )
            .child(
                v_flex()
                    .gap_3()
                    .p_3()
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().sidebar.alpha(0.45))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::new(IconName::Bot))
                            .child(div().font_semibold().child("Execution")),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("start-processing")
                                    .success()
                                    .icon(IconName::Bot)
                                    .label("Start processing")
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.run_dummy_processing(window, cx);
                                    })),
                            )
                            .child(
                                Button::new("advance-progress")
                                    .icon(IconName::Plus)
                                    .label("Advance progress")
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.advance_dummy_progress(cx);
                                    })),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(format!(
                                "Loading images: {} / {}",
                                self.processing.count_loaded,
                                self.processing.count_lights + self.processing.count_darks
                            )))
                            .child(Progress::new().value(self.loading_percent())),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(format!(
                                "Merging images: {} / {}",
                                self.processing.count_merged,
                                self.processing.count_lights.saturating_sub(1)
                                    + self.processing.count_darks.saturating_sub(1)
                            )))
                            .child(Progress::new().value(self.merge_percent())),
                    ),
            )
    }

    pub(crate) fn render_preview_step(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(preview) = &self.preview {
            return v_flex()
                .items_center()
                .gap_2()
                .p_3()
                .rounded(cx.theme().radius)
                .bg(cx.theme().sidebar.alpha(0.45))
                .border_1()
                .border_color(cx.theme().border)
                .child(img(preview.image_url).max_w_full().max_h(px(760.0)))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(preview.exif),
                );
        }

        v_flex().gap_3().child(
            h_flex()
                .gap_2()
                .items_center()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(Icon::new(IconName::Info).small())
                .child("Run a dummy merge in step 3 to generate an example preview."),
        )
    }
}
