use gpui::{
    Context, InteractiveElement as _, IntoElement, MouseButton, ObjectFit, ParentElement as _,
    Styled as _, StyledImage as _, div, img, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    progress::Progress,
    v_flex,
};

use crate::ui::{
    app::Trawls,
    model::{LightframeStats, MergeMode},
};

use super::{format_duration, format_file_size, light_dark};
use super::image_viewer::{ImageViewerParams, render_image_viewer};

impl Trawls {
    pub(crate) fn render_image_step(
        &self,
        is_light: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if self.frame_preview.is_some() || self.frame_preview_loading {
            return self.render_frame_preview(cx);
        }

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
        let has_error_warning = Self::error_warning(&rows);
        let has_setting_warning = Self::camera_setting_warning(&rows);
        let has_interval_warning = is_light && Self::interval_warning(&rows);

        v_flex()
            .h_full()
            .min_h_0()
            .gap_4()
            .when(has_error_warning, |this| {
                this.child(
                    Alert::warning(
                        light_dark(is_light, "warn-light-error", "warn-dark-error"),
                        "The metadata of some files could not be loaded.",
                    )
                    .banner(),
                )
            })
            .when(has_setting_warning, |this| {
                this.child(
                    Alert::warning(
                        light_dark(is_light, "warn-light-settings", "warn-dark-settings"),
                        "The camera settings between some frames changed.",
                    )
                    .banner(),
                )
            })
            .when(has_interval_warning, |this| {
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
                                Button::new(light_dark(is_light, "pick-lights", "pick-darks"))
                                .success()
                                .icon(IconName::Search)
                                .label("Add images")
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.pick_image_files(is_light, window, cx);
                                    },
                                )),
                            )
                            .child(
                                Button::new(light_dark(is_light, "clear-lights", "clear-darks"))
                                .warning()
                                .icon(IconName::Minus)
                                .label("Clear list")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.clear_frames(is_light, cx);
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
            .child(self.render_frame_table(rows, is_light, is_light, cx))
            .into_any_element()
    }

    fn render_lightframe_stats(
        &self,
        stats: LightframeStats,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let idle_seconds = stats.non_exposing_seconds;
        let idle_display = format!(
            "{} idle ({})",
            format_duration(idle_seconds),
            if stats.total_time_seconds > 0.0 {
                format!(
                    "{:.0}%",
                    (idle_seconds / stats.total_time_seconds * 100.0).clamp(0.0, 100.0)
                )
            } else {
                "0%".to_string()
            }
        );

        h_flex()
            .gap_4()
            .flex_wrap()
            .p_3()
            .rounded(cx.theme().radius)
            .bg(cx.theme().sidebar.alpha(0.45))
            .border_1()
            .border_color(cx.theme().border)
            .child(self.render_stat_cell("Frames", format!(
                "{} total, {} valid, {} errors",
                stats.total_count, stats.valid_count, stats.error_count
            ), cx))
            .child(self.render_stat_cell(
                "Shutter open",
                format_duration(stats.total_exposure_seconds),
                cx,
            ))
            .child(self.render_stat_cell(
                "Session span",
                format_duration(stats.total_time_seconds),
                cx,
            ))
            .child(self.render_stat_cell(
                "Efficiency",
                format!("{:.0}%", stats.duty_cycle_percent),
                cx,
            ))
            .child(self.render_stat_cell("Gaps", idle_display, cx))
            .child(match &stats.largest_gap {
                Some(gap) => self.render_stat_cell(
                    "Largest gap",
                    format!("{} ({} -> {})", format_duration(gap.seconds), gap.from, gap.to),
                    cx,
                ),
                None => self.render_stat_cell("Largest gap", "none".to_string(), cx),
            })
    }

    fn render_stat_cell(
        &self,
        label: &str,
        value: String,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .gap_0p5()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(label.to_string()),
            )
            .child(div().text_sm().child(value))
    }

    fn render_mode_card(
        &self,
        id: &'static str,
        image: &str,
        title: &str,
        description: &str,
        mode: MergeMode,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.merge_mode == mode;
        div()
            .id(id)
            .relative()
            .flex_1()
            .min_w(px(360.0))
            .h(px(190.0))
            .overflow_hidden()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(if selected {
                cx.theme().primary
            } else {
                cx.theme().border
            })
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.merge_mode = mode;
                    cx.notify();
                }),
            )
            .child(
                img(image.to_string())
                    .size_full()
                    .object_fit(ObjectFit::Cover),
            )
            .child(
                v_flex()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .p_2()
                    .bg(cx.theme().background.alpha(0.6))
                    .child(div().text_sm().font_semibold().child(title.to_string()))
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(
                                Icon::new(if selected {
                                    IconName::CircleCheck
                                } else {
                                    IconName::CircleUser
                                })
                                .xsmall(),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .child(description.to_string()),
                            ),
                    ),
            )
    }

    pub(crate) fn render_processing_step(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child(Icon::new(IconName::FolderOpen))
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
                                        this.choose_output(window, cx);
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
                        h_flex()
                            .gap_4()
                            .flex_wrap()
                            .child(self.render_mode_card(
                                "mode-normal-card",
                                "examples/mode_normal.jpg",
                                "Normal",
                                "All images will be weighted identically.",
                                MergeMode::Normal,
                                cx,
                            ))
                            .child(self.render_mode_card(
                                "mode-falling-card",
                                "examples/mode_falling.jpg",
                                "Falling Comets",
                                "Later images become darker, such that stars will fade away.",
                                MergeMode::Falling,
                                cx,
                            ))
                            .child(self.render_mode_card(
                                "mode-raising-card",
                                "examples/mode_raising.jpg",
                                "Raising Comets",
                                "Earlier images are darker, such that stars will fade in.",
                                MergeMode::Raising,
                                cx,
                            )),
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
                            .child(Icon::new(IconName::Loader))
                            .child(div().font_semibold().child("Execution")),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("start-processing")
                                    .success()
                                    .icon(IconName::Loader)
                                    .label(if self.processing_running {
                                        "Processing..."
                                    } else {
                                        "Start processing"
                                    })
                                    .disabled(self.processing_running)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.start_processing(window, cx);
                                    })),
                            )
                            .when(self.processing_running, |el| {
                                el.child(
                                    Button::new("cancel-processing")
                                        .warning()
                                        .small()
                                        .icon(IconName::Close)
                                        .label("Cancel")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.cancel_processing(cx);
                                        })),
                                )
                            }),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(format!(
                                "Loading: {} / {} ({:.0}%)",
                                self.processing.count_loaded,
                                self.processing.count_lights + self.processing.count_darks,
                                self.loading_percent(),
                            )))
                            .child(Progress::new().value(self.loading_percent())),
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(div().text_sm().child(format!(
                                "Merging: {} / {} ({:.0}%)",
                                self.processing.count_merged,
                                self.processing.count_lights.saturating_sub(1)
                                    + self.processing.count_darks.saturating_sub(1),
                                self.merge_percent(),
                            )))
                            .child(Progress::new().value(self.merge_percent())),
                    )
                    .when_some(self.processing_elapsed_seconds(), |el, elapsed| {
                        el.child(
                            h_flex()
                                .gap_4()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "{}: {}",
                                    if self.processing_running { "Elapsed" } else { "Completed in" },
                                    format_duration(elapsed),
                                ))
                                .when(self.processing_running, |el| {
                                    if let Some(eta) = self.processing_eta_seconds() {
                                        el.child(format!("ETA: ~{}", format_duration(eta)))
                                    } else {
                                        el
                                    }
                                }),
                        )
                    }),
            )
    }

    pub(crate) fn render_preview_step(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(preview) = &self.preview {
            let info_bar = h_flex()
                .gap_4()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(preview.exif.clone())
                .child(format!("{} frames", preview.frame_count))
                .child(format!(
                    "Total exposure: {}",
                    format_duration(preview.total_exposure_seconds),
                ))
                .child(format!("Mode: {}", preview.merge_mode.label()))
                .when(preview.output_file_size > 0, |el| {
                    el.child(format!(
                        "Size: {}",
                        format_file_size(preview.output_file_size),
                    ))
                })
                .child(format!("Saved to: {}", preview.output_path));

            return render_image_viewer(
                ImageViewerParams {
                    id_prefix: "result-preview",
                    image_source: img(preview.image.clone())
                        .size_full()
                        .object_fit(ObjectFit::Contain)
                        .into_any_element(),
                    image_width: preview.width_px,
                    image_height: preview.height_px,
                    info_bar: Some(info_bar.into_any_element()),
                    left_header: None,
                },
                &self.preview_viewer,
                |t| &mut t.preview_viewer,
                cx,
            );
        }

        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(Icon::new(IconName::Info).small())
                    .child("Run a merge in step 3 to generate a preview."),
            )
            .into_any_element()
    }

    fn render_frame_preview(&self, cx: &mut Context<Self>) -> gpui::AnyElement {
        if self.frame_preview_loading {
            return v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .gap_2()
                .child(Icon::new(IconName::LoaderCircle).text_color(cx.theme().muted_foreground))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Loading preview..."),
                )
                .into_any_element();
        }

        let Some(preview) = &self.frame_preview else {
            return div().into_any_element();
        };

        let left_header = h_flex()
            .gap_2()
            .items_center()
            .child(
                Button::new("frame-preview-close")
                    .small()
                    .icon(IconName::ArrowLeft)
                    .label("Back")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.close_frame_preview(cx);
                    })),
            )
            .child(div().font_semibold().child(preview.filename.clone()));

        let info_bar = div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(format!(
                "{}  |  {}x{}",
                preview.exif_summary, preview.width_px, preview.height_px
            ));

        render_image_viewer(
            ImageViewerParams {
                id_prefix: "frame-preview",
                image_source: img(preview.image.clone())
                    .size_full()
                    .object_fit(ObjectFit::Contain)
                    .into_any_element(),
                image_width: preview.width_px,
                image_height: preview.height_px,
                info_bar: Some(info_bar.into_any_element()),
                left_header: Some(left_header.into_any_element()),
            },
            &self.frame_preview_viewer,
            |t| &mut t.frame_preview_viewer,
            cx,
        )
    }
}
