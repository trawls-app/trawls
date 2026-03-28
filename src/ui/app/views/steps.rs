use gpui::{
    Context, InteractiveElement as _, IntoElement, Keystroke, MouseButton, ObjectFit,
    ParentElement as _, ScrollDelta, ScrollWheelEvent, StatefulInteractiveElement as _,
    Styled as _, StyledImage as _, div, img, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    kbd::Kbd,
    progress::Progress,
    scroll::{Scrollbar, ScrollbarShow},
    v_flex,
};

use crate::ui::{
    app::TrawlsDummy,
    model::{LightframeStats, MergeMode, PreviewImageSource},
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
            .h_full()
            .min_h_0()
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
                                .label("Add images")
                                .on_click(cx.listener(
                                    move |this, _, window, cx| {
                                        this.pick_image_files(is_light, window, cx);
                                    },
                                )),
                            )
                            .child(
                                Button::new(if is_light {
                                    "add-dummy-lights"
                                } else {
                                    "add-dummy-darks"
                                })
                                .icon(IconName::Plus)
                                .label("Add dummy frames")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.queue_mock_metadata_loading(is_light, cx);
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
                        h_flex()
                            .gap_4()
                            .flex_wrap()
                            .child(
                                v_flex()
                                    .id("mode-normal-card")
                                    .flex_1()
                                    .min_w(px(360.0))
                                    .overflow_hidden()
                                    .rounded(cx.theme().radius)
                                    .border_1()
                                    .border_color(if self.merge_mode == MergeMode::Normal {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().border
                                    })
                                    .bg(if self.merge_mode == MergeMode::Normal {
                                        cx.theme().primary.alpha(0.08)
                                    } else {
                                        cx.theme().sidebar.alpha(0.35)
                                    })
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.merge_mode = MergeMode::Normal;
                                            cx.notify();
                                        }),
                                    )
                                    .child(
                                        img("examples/mode_normal.jpg")
                                            .w_full()
                                            .h(px(190.0))
                                            .object_fit(ObjectFit::Cover),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .p_4()
                                            .child(div().text_xl().font_semibold().child("Normal"))
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .items_start()
                                                    .child(
                                                        Icon::new(if self.merge_mode == MergeMode::Normal {
                                                            IconName::CircleCheck
                                                        } else {
                                                            IconName::CircleUser
                                                        })
                                                        .small(),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .whitespace_normal()
                                                            .text_base()
                                                            .child("All images will be weighted identically."),
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .id("mode-falling-card")
                                    .flex_1()
                                    .min_w(px(360.0))
                                    .overflow_hidden()
                                    .rounded(cx.theme().radius)
                                    .border_1()
                                    .border_color(if self.merge_mode == MergeMode::Falling {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().border
                                    })
                                    .bg(if self.merge_mode == MergeMode::Falling {
                                        cx.theme().primary.alpha(0.08)
                                    } else {
                                        cx.theme().sidebar.alpha(0.35)
                                    })
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.merge_mode = MergeMode::Falling;
                                            cx.notify();
                                        }),
                                    )
                                    .child(
                                        img("examples/mode_falling.jpg")
                                            .w_full()
                                            .h(px(190.0))
                                            .object_fit(ObjectFit::Cover),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .p_4()
                                            .child(
                                                div()
                                                    .text_xl()
                                                    .font_semibold()
                                                    .child("Falling Comets"),
                                            )
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .items_start()
                                                    .child(
                                                        Icon::new(if self.merge_mode == MergeMode::Falling {
                                                            IconName::CircleCheck
                                                        } else {
                                                            IconName::CircleUser
                                                        })
                                                        .small(),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .whitespace_normal()
                                                            .text_base()
                                                            .child("Later images become darker, such that stars will fade away."),
                                                    ),
                                            ),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .id("mode-raising-card")
                                    .flex_1()
                                    .min_w(px(360.0))
                                    .overflow_hidden()
                                    .rounded(cx.theme().radius)
                                    .border_1()
                                    .border_color(if self.merge_mode == MergeMode::Raising {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().border
                                    })
                                    .bg(if self.merge_mode == MergeMode::Raising {
                                        cx.theme().primary.alpha(0.08)
                                    } else {
                                        cx.theme().sidebar.alpha(0.35)
                                    })
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|this, _, _, cx| {
                                            this.merge_mode = MergeMode::Raising;
                                            cx.notify();
                                        }),
                                    )
                                    .child(
                                        img("examples/mode_raising.jpg")
                                            .w_full()
                                            .h(px(190.0))
                                            .object_fit(ObjectFit::Cover),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_2()
                                            .p_4()
                                            .child(
                                                div()
                                                    .text_xl()
                                                    .font_semibold()
                                                    .child("Raising Comets"),
                                            )
                                            .child(
                                                h_flex()
                                                    .gap_2()
                                                    .items_start()
                                                    .child(
                                                        Icon::new(if self.merge_mode == MergeMode::Raising {
                                                            IconName::CircleCheck
                                                        } else {
                                                            IconName::CircleUser
                                                        })
                                                        .small(),
                                                    )
                                                    .child(
                                                        div()
                                                            .flex_1()
                                                            .whitespace_normal()
                                                            .text_base()
                                                            .child("Earlier images are darker, such that stars will fade in."),
                                                    ),
                                            ),
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
                                    .label(if self.processing_running {
                                        "Processing..."
                                    } else {
                                        "Start processing"
                                    })
                                    .disabled(self.processing_running)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.run_dummy_processing(window, cx);
                                    })),
                            )
                            .when(self.processing_running, |this| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("Simulating background processing..."),
                                )
                            }),
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
                                            this.zoom_preview_out(cx);
                                        })),
                                )
                                .child(
                                    Button::new("preview-zoom-fit")
                                        .small()
                                        .label("Fit to screen")
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.fit_preview_to_window(window, cx);
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
                                            this.zoom_preview_in(cx);
                                        })),
                                ),
                        )
                        .child(
                            h_flex()
                                .gap_1()
                                .items_center()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child("Hint: hold")
                                .child(Kbd::new(
                                    Keystroke::parse(if cfg!(target_os = "macos") {
                                        "cmd"
                                    } else {
                                        "ctrl"
                                    })
                                    .expect("valid keystroke"),
                                ))
                                .child("and scroll to zoom."),
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
                        .id("preview-image-panel")
                        .relative()
                        .flex_1()
                        .min_h_0()
                        .w_full()
                        .scrollbar_width(px(12.0))
                        .on_scroll_wheel(cx.listener(
                            |this, event: &ScrollWheelEvent, window, cx| {
                                if !event.modifiers.secondary() {
                                    return;
                                }

                                let delta_y = match event.delta {
                                    ScrollDelta::Pixels(delta) => f32::from(delta.y),
                                    ScrollDelta::Lines(delta) => delta.y * 16.0,
                                };

                                if delta_y < 0.0 {
                                    this.zoom_preview_at_cursor(event.position, true, window, cx);
                                } else if delta_y > 0.0 {
                                    this.zoom_preview_at_cursor(event.position, false, window, cx);
                                }

                                cx.stop_propagation();
                            },
                        ))
                        .child(
                            div()
                                .id("preview-image-scroll-content")
                                .track_scroll(&self.preview_scroll_handle)
                                .overflow_x_scroll()
                                .overflow_y_scroll()
                                .size_full()
                                .child(
                                    div().w(px(zoomed_w)).h(px(zoomed_h)).flex_shrink_0().child(
                                        match &preview.image_source {
                                            PreviewImageSource::Embedded(path) => img(path.clone())
                                                .size_full()
                                                .object_fit(ObjectFit::Contain)
                                                .into_any_element(),
                                            PreviewImageSource::FilePath(path) => img(path.clone())
                                                .size_full()
                                                .object_fit(ObjectFit::Contain)
                                                .into_any_element(),
                                        },
                                    ),
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
                                    Scrollbar::both(
                                        &self.preview_scroll_state,
                                        &self.preview_scroll_handle,
                                    )
                                    .scrollbar_show(ScrollbarShow::Always),
                                ),
                        ),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(preview.exif.clone()),
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
