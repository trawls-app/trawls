use gpui::{
    Context, InteractiveElement as _, IntoElement, ObjectFit, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, StyledImage as _, div, img,
    prelude::FluentBuilder as _, px,
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

use crate::ui::{
    app::TrawlsDummy,
    model::{LightframeStats, MergeMode},
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
        let light_stats = if is_light {
            Self::lightframe_stats(&rows)
        } else {
            None
        };

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
                                            this.selected_light_paths.clear();
                                        } else {
                                            this.darkframes = sample_darkframes();
                                            this.selected_dark_paths.clear();
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
                                            this.selected_light_paths.clear();
                                        } else {
                                            this.darkframes.clear();
                                            this.selected_dark_paths.clear();
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
            .when_some(light_stats, |this, stats| {
                this.child(self.render_lightframe_stats(stats, cx))
            })
            .child(self.render_frame_table(&rows, is_light, is_light, cx))
    }

    fn render_lightframe_stats(
        &self,
        stats: LightframeStats,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_2()
            .p_3()
            .rounded(cx.theme().radius)
            .bg(cx.theme().sidebar.alpha(0.45))
            .border_1()
            .border_color(cx.theme().border)
            .child(div().font_semibold().child("Lightframe statistics"))
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Frames"),
                            )
                            .child(div().text_sm().child(format!(
                                "{} total, {} valid, {} errors",
                                stats.total_count, stats.valid_count, stats.error_count
                            ))),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Total exposure"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!("{:.1} s", stats.total_exposure_seconds)),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Total time"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!("{:.1} s", stats.total_time_seconds)),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Time not exposing"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!("{:.1} s", stats.non_exposing_seconds)),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Duty cycle"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!("{:.1}%", stats.duty_cycle_percent)),
                            ),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Not exposing"),
                            )
                            .child(div().text_sm().child(format!(
                                "{:.1}%",
                                if stats.total_time_seconds > 0.0 {
                                    (stats.non_exposing_seconds / stats.total_time_seconds * 100.0)
                                        .clamp(0.0, 100.0)
                                } else {
                                    0.0
                                }
                            ))),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Largest gap"),
                            )
                            .child(match &stats.largest_gap {
                                Some(gap) => div().text_sm().child(format!(
                                    "{:.1} s ({} -> {})",
                                    gap.seconds, gap.from, gap.to
                                )),
                                None => div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("n/a"),
                            }),
                    ),
            )
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
            let zoomed_w = preview.width_px as f32 * self.preview_zoom;
            let zoomed_h = preview.height_px as f32 * self.preview_zoom;
            return v_flex()
                .w_full()
                .h_full()
                .gap_3()
                .p_3()
                .rounded(cx.theme().radius)
                .bg(cx.theme().sidebar.alpha(0.45))
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    h_flex()
                        .justify_between()
                        .items_center()
                        .gap_2()
                        .child(
                            h_flex()
                                .gap_2()
                                .items_center()
                                .child(
                                    Button::new("preview-zoom-out")
                                        .small()
                                        .icon(IconName::Minus)
                                        .label("Zoom out")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_preview_zoom(this.preview_zoom - 0.1, cx);
                                        })),
                                )
                                .child(
                                    Button::new("preview-zoom-reset")
                                        .small()
                                        .label("Reset")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_preview_zoom(1.0, cx);
                                        })),
                                )
                                .child(
                                    Button::new("preview-zoom-in")
                                        .small()
                                        .icon(IconName::Plus)
                                        .label("Zoom in")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.set_preview_zoom(this.preview_zoom + 0.1, cx);
                                        })),
                                ),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{:.0}%", self.preview_zoom * 100.0)),
                        ),
                )
                .child(
                    v_flex()
                        .id("preview-image-scroll")
                        .flex_1()
                        .h_full()
                        .w_full()
                        .overflow_x_scroll()
                        .overflow_y_scroll()
                        .child(
                            div().w(px(zoomed_w)).h(px(zoomed_h)).flex_shrink_0().child(
                                img(preview.image_url)
                                    .size_full()
                                    .object_fit(ObjectFit::Contain),
                            ),
                        ),
                )
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
