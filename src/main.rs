use gpui::{
    App, AppContext, Application, AssetSource, Context, Entity, FocusHandle, Focusable,
    InteractiveElement as _, IntoElement, ParentElement as _, Render, SharedString,
    StatefulInteractiveElement as _, Styled as _, Window, WindowOptions, div, img,
    prelude::FluentBuilder as _, px, relative,
};
use gpui_component::{
    ActiveTheme as _, Icon, IconName, Root, Sizable as _, StyledExt as _, Theme, ThemeMode,
    ThemeRegistry, TitleBar, WindowExt as _,
    alert::Alert,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    progress::Progress,
    sidebar::{Sidebar, SidebarFooter, SidebarGroup, SidebarHeader, SidebarMenu, SidebarMenuItem},
    tab::{Tab, TabBar},
    v_flex,
};
use rust_embed::RustEmbed;
use std::borrow::Cow;
use std::path::PathBuf;

#[derive(RustEmbed)]
#[folder = "./assets"]
#[include = "examples/**"]
#[include = "icons/**"]
#[include = "*.png"]
struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> anyhow::Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Ok(Assets::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> anyhow::Result<Vec<SharedString>> {
        Ok(Assets::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

#[derive(Clone)]
struct FrameInfo {
    path: &'static str,
    filename: &'static str,
    exposure_seconds: f32,
    interval_seconds: Option<f32>,
    aperture: f32,
    iso: u32,
    creation_time: &'static str,
    error: Option<&'static str>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortKey {
    CreationTime,
    Filename,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MergeMode {
    Normal,
    Falling,
    Raising,
}

#[derive(Clone)]
struct PreviewData {
    image_url: &'static str,
    exif: &'static str,
}

#[derive(Clone)]
struct ProcessingMock {
    count_lights: usize,
    count_darks: usize,
    count_loaded: usize,
    count_loading: usize,
    count_merged: usize,
    count_merging: usize,
}

impl Default for ProcessingMock {
    fn default() -> Self {
        Self {
            count_lights: 0,
            count_darks: 0,
            count_loaded: 0,
            count_loading: 0,
            count_merged: 0,
            count_merging: 0,
        }
    }
}

struct TrawlsDummy {
    focus_handle: FocusHandle,
    active_step: usize,
    lightframes: Vec<FrameInfo>,
    darkframes: Vec<FrameInfo>,
    light_sort: SortKey,
    dark_sort: SortKey,
    output_path: Option<SharedString>,
    output_input: Entity<InputState>,
    merge_mode: MergeMode,
    processing: ProcessingMock,
    preview: Option<PreviewData>,
}

impl TrawlsDummy {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let output_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("No output path selected"));

        Self {
            focus_handle: cx.focus_handle(),
            active_step: 0,
            lightframes: Vec::new(),
            darkframes: Vec::new(),
            light_sort: SortKey::CreationTime,
            dark_sort: SortKey::CreationTime,
            output_path: None,
            output_input,
            merge_mode: MergeMode::Normal,
            processing: ProcessingMock::default(),
            preview: None,
        }
    }

    fn step_title(&self) -> &'static str {
        match self.active_step {
            0 => "1. Add Lightframes",
            1 => "2. Add Darkframes",
            2 => "3. Process images",
            _ => "4. Preview",
        }
    }

    fn step_description(&self) -> &'static str {
        match self.active_step {
            0 => "Select the lightframes in this step.",
            1 => "Optional: add darkframes with matching settings to reduce noise.",
            2 => "Select processing options and run the merge.",
            _ => "Inspect the generated preview after processing.",
        }
    }

    fn sorted_frames(frames: &[FrameInfo], sort: SortKey) -> Vec<FrameInfo> {
        let mut rows: Vec<FrameInfo> = frames.to_vec();
        rows.sort_by(|a, b| {
            if a.error.is_some() && b.error.is_none() {
                return std::cmp::Ordering::Less;
            }
            if a.error.is_none() && b.error.is_some() {
                return std::cmp::Ordering::Greater;
            }

            match sort {
                SortKey::CreationTime => a.creation_time.cmp(b.creation_time),
                SortKey::Filename => a.filename.cmp(b.filename),
            }
        });
        rows
    }

    fn error_warning(frames: &[FrameInfo]) -> bool {
        frames.iter().any(|f| f.error.is_some())
    }

    fn camera_setting_warning(frames: &[FrameInfo]) -> bool {
        let mut apertures = std::collections::BTreeSet::new();
        let mut exposures = std::collections::BTreeSet::new();
        let mut isos = std::collections::BTreeSet::new();

        for frame in frames.iter().filter(|f| f.error.is_none()) {
            apertures.insert(format!("{:.1}", frame.aperture));
            exposures.insert(format!("{:.1}", frame.exposure_seconds));
            isos.insert(frame.iso);
        }

        apertures.len() > 1 || exposures.len() > 1 || isos.len() > 1
    }

    fn interval_warning_threshold(frames: &[FrameInfo]) -> Option<f32> {
        let mut sum = 0.0;
        let mut count = 0;
        for frame in frames {
            if let Some(dt) = frame.interval_seconds {
                sum += dt.abs();
                count += 1;
            }
        }

        if count > 0 {
            Some((sum / count as f32) + 1.0)
        } else {
            None
        }
    }

    fn interval_warning(frames: &[FrameInfo]) -> bool {
        if let Some(threshold) = Self::interval_warning_threshold(frames) {
            return frames
                .iter()
                .any(|f| f.interval_seconds.is_some_and(|v| v.abs() > threshold));
        }

        false
    }

    fn choose_dummy_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let next = if self.output_path.is_some() {
            "~/Pictures/startrail_result.dng"
        } else {
            "~/Pictures/trawls_output.dng"
        };

        self.output_path = Some(next.into());
        self.output_input.update(cx, |state, cx| {
            state.set_value(next, window, cx);
        });
        cx.notify();
    }

    fn clear_output(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.output_path = None;
        self.output_input.update(cx, |state, cx| {
            state.set_value("", window, cx);
        });
        cx.notify();
    }

    fn run_dummy_processing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let mut issues: Vec<String> = Vec::new();

        if Self::error_warning(&self.lightframes) || Self::error_warning(&self.darkframes) {
            issues.push("The metadata of some files could not be loaded.".to_string());
        }
        if self.lightframes.len() < 2 {
            issues.push("Less than two lightframes are selected.".to_string());
        }
        if self.output_path.is_none() {
            issues.push("No output path is specified.".to_string());
        }

        if !issues.is_empty() {
            window.open_dialog(cx, move |dialog, _, _| {
                dialog
                    .title("Can't start processing")
                    .child(
                        v_flex().gap_2().children(
                            issues
                                .iter()
                                .map(|item| div().text_sm().child(format!("- {item}"))),
                        ),
                    )
                    .footer(|_, _, _, _| {
                        vec![
                            Button::new("ready-ok")
                                .primary()
                                .label("OK")
                                .on_click(|_, window, cx| window.close_dialog(cx)),
                        ]
                    })
            });
            return;
        }

        let count_lights = self.lightframes.len();
        let count_darks = self.darkframes.len();
        let merge_total = count_lights.saturating_sub(1) + count_darks.saturating_sub(1);

        self.processing = ProcessingMock {
            count_lights,
            count_darks,
            count_loaded: count_lights + count_darks,
            count_loading: 0,
            count_merged: merge_total.saturating_sub(1),
            count_merging: usize::from(merge_total > 0),
        };

        self.preview = Some(PreviewData {
            image_url: "https://images.unsplash.com/photo-1462331940025-496dfbfc7564?w=1600",
            exif: "ISO 1600, f/2.8, 20s",
        });
        self.active_step = 3;

        window.push_notification("Dummy processing finished and preview generated.", cx);
        cx.notify();
    }

    fn advance_dummy_progress(&mut self, cx: &mut Context<Self>) {
        let loading_total = self.processing.count_lights + self.processing.count_darks;
        if self.processing.count_loaded < loading_total {
            self.processing.count_loading = 1;
            self.processing.count_loaded = (self.processing.count_loaded + 1).min(loading_total);
            if self.processing.count_loaded == loading_total {
                self.processing.count_loading = 0;
            }
        }

        let merge_total = self.processing.count_lights.saturating_sub(1)
            + self.processing.count_darks.saturating_sub(1);
        if self.processing.count_merged < merge_total {
            self.processing.count_merging = 1;
            self.processing.count_merged = (self.processing.count_merged + 1).min(merge_total);
            if self.processing.count_merged == merge_total {
                self.processing.count_merging = 0;
            }
        }

        cx.notify();
    }

    fn loading_percent(&self) -> f32 {
        let total = (self.processing.count_lights + self.processing.count_darks).max(1) as f32;
        let current =
            self.processing.count_loaded as f32 + (self.processing.count_loading as f32 * 0.5);
        (current / total * 100.0).clamp(0.0, 100.0)
    }

    fn merge_percent(&self) -> f32 {
        let total = (self.processing.count_lights.saturating_sub(1)
            + self.processing.count_darks.saturating_sub(1))
        .max(1) as f32;
        let current =
            self.processing.count_merged as f32 + (self.processing.count_merging as f32 * 0.5);
        (current / total * 100.0).clamp(0.0, 100.0)
    }

    fn render_titlebar(&self, _cx: &Context<Self>) -> impl IntoElement {
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

    fn render_frame_table(
        &self,
        rows: &[FrameInfo],
        show_interval: bool,
        is_light: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let threshold = Self::interval_warning_threshold(rows).unwrap_or(9999.0);
        let active_sort = if is_light {
            self.light_sort
        } else {
            self.dark_sort
        };
        let filename_label = if active_sort == SortKey::Filename {
            "Filename  ^"
        } else {
            "Filename"
        };
        let time_label = if active_sort == SortKey::CreationTime {
            "Time  ^"
        } else {
            "Time"
        };

        v_flex()
            .w_full()
            .gap_0()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .child(
                h_flex()
                    .w_full()
                    .px_2()
                    .py_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().sidebar_accent.alpha(0.7))
                    .font_semibold()
                    .text_xs()
                    .child(
                        div().w(px(260.)).child(
                            Button::new(if is_light {
                                "header-sort-light-filename"
                            } else {
                                "header-sort-dark-filename"
                            })
                            .ghost()
                            .xsmall()
                            .label(filename_label)
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if is_light {
                                        this.light_sort = SortKey::Filename;
                                    } else {
                                        this.dark_sort = SortKey::Filename;
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                    )
                    .child(div().w(px(90.)).text_center().child("Exposure"))
                    .when(show_interval, |this| {
                        this.child(div().w(px(90.)).text_center().child("Interval"))
                    })
                    .child(div().w(px(90.)).text_center().child("Aperture"))
                    .child(div().w(px(90.)).text_center().child("ISO"))
                    .child(
                        div().w(px(210.)).child(
                            Button::new(if is_light {
                                "header-sort-light-time"
                            } else {
                                "header-sort-dark-time"
                            })
                            .ghost()
                            .xsmall()
                            .label(time_label)
                            .on_click(cx.listener(
                                move |this, _, _, cx| {
                                    if is_light {
                                        this.light_sort = SortKey::CreationTime;
                                    } else {
                                        this.dark_sort = SortKey::CreationTime;
                                    }
                                    cx.notify();
                                },
                            )),
                        ),
                    )
                    .child(div().w(px(85.)).text_right().child("Action")),
            )
            .child(
                v_flex()
                    .id("frame-table-scroll")
                    .max_h(px(480.0))
                    .overflow_y_scroll()
                    .child(v_flex().children(rows.iter().enumerate().map(|(ix, row)| {
                        if let Some(error) = row.error {
                            return h_flex()
                                .w_full()
                                .px_2()
                                .py_1()
                                .border_b_1()
                                .border_color(cx.theme().border)
                                .bg(cx.theme().danger.alpha(0.18))
                                .text_xs()
                                .child(div().w(px(260.)).child(row.filename))
                                .child(div().w(px(570.)).child(error))
                                .child(
                                    div().w(px(85.)).text_right().child(
                                        Button::new(("remove-err", ix))
                                            .small()
                                            .danger()
                                            .icon(IconName::Minus)
                                            .label("Remove")
                                            .on_click(cx.listener({
                                                let path = row.path.to_string();
                                                move |this, _, _, cx| {
                                                    if is_light {
                                                        this.lightframes.retain(|f| f.path != path);
                                                    } else {
                                                        this.darkframes.retain(|f| f.path != path);
                                                    }
                                                    cx.notify();
                                                }
                                            })),
                                    ),
                                )
                                .into_any_element();
                        }

                        h_flex()
                            .w_full()
                            .px_2()
                            .py_1()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .when(ix % 2 == 1, |this| {
                                this.bg(cx.theme().sidebar_accent.alpha(0.35))
                            })
                            .text_xs()
                            .child(div().w(px(260.)).child(row.filename))
                            .child(
                                div()
                                    .w(px(90.))
                                    .text_center()
                                    .child(format!("{:.1}s", row.exposure_seconds)),
                            )
                            .when(show_interval, |this| {
                                this.child(
                                    div()
                                        .w(px(90.))
                                        .text_center()
                                        .when(
                                            row.interval_seconds
                                                .is_some_and(|value| value.abs() > threshold),
                                            |this| this.bg(cx.theme().warning.alpha(0.2)),
                                        )
                                        .child(match row.interval_seconds {
                                            Some(value) => format!("{value:.1}s"),
                                            None => "-".to_string(),
                                        }),
                                )
                            })
                            .child(
                                div()
                                    .w(px(90.))
                                    .text_center()
                                    .child(format!("f/{:.1}", row.aperture)),
                            )
                            .child(div().w(px(90.)).text_center().child(format!("{}", row.iso)))
                            .child(div().w(px(210.)).text_center().child(row.creation_time))
                            .child(
                                div().w(px(85.)).text_right().child(
                                    Button::new(("remove", ix))
                                        .small()
                                        .icon(IconName::Minus)
                                        .label("Remove")
                                        .on_click(cx.listener({
                                            let path = row.path.to_string();
                                            move |this, _, _, cx| {
                                                if is_light {
                                                    this.lightframes.retain(|f| f.path != path);
                                                } else {
                                                    this.darkframes.retain(|f| f.path != path);
                                                }
                                                cx.notify();
                                            }
                                        })),
                                ),
                            )
                            .into_any_element()
                    }))),
            )
    }

    fn render_image_step(&self, is_light: bool, cx: &mut Context<Self>) -> impl IntoElement {
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

    fn render_processing_step(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

    fn render_preview_step(&self, cx: &mut Context<Self>) -> impl IntoElement {
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

impl Focusable for TrawlsDummy {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TrawlsDummy {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.active_step {
            0 => self.render_image_step(true, cx).into_any_element(),
            1 => self.render_image_step(false, cx).into_any_element(),
            2 => self.render_processing_step(cx).into_any_element(),
            _ => self.render_preview_step(cx).into_any_element(),
        };

        div()
            .id("trawls-dummy")
            .size_full()
            .track_focus(&self.focus_handle)
            .bg(cx.theme().background)
            .child(
                v_flex().size_full().child(self.render_titlebar(cx)).child(
                    h_flex()
                        .flex_1()
                        .h_full()
                        .items_start()
                        .overflow_hidden()
                        .child(
                            Sidebar::left()
                                .w(px(250.0))
                                .h_full()
                                .border_r_1()
                                .border_color(cx.theme().border)
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
                                                .child(
                                                    img("examples/trawls-logo.png")
                                                        .w(px(22.0))
                                                        .h(px(22.0)),
                                                ),
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
                                                SidebarMenuItem::new("Add Lightframes")
                                                    .icon(IconName::SquareTerminal)
                                                    .active(self.active_step == 0)
                                                    .suffix(format!("{}", self.lightframes.len()))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.active_step = 0;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                SidebarMenuItem::new("Add Darkframes")
                                                    .icon(IconName::Bot)
                                                    .active(self.active_step == 1)
                                                    .suffix(format!("{}", self.darkframes.len()))
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.active_step = 1;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                SidebarMenuItem::new("Process images")
                                                    .icon(IconName::Settings2)
                                                    .active(self.active_step == 2)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.active_step = 2;
                                                        cx.notify();
                                                    })),
                                            )
                                            .child(
                                                SidebarMenuItem::new("Preview")
                                                    .icon(IconName::BookOpen)
                                                    .active(self.active_step == 3)
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.active_step = 3;
                                                        cx.notify();
                                                    })),
                                            ),
                                    ),
                                )
                                .footer(
                                    SidebarFooter::new().child(
                                        Button::new("sidebar-version-link")
                                            .link()
                                            .label("trawls v0.5.0")
                                            .on_click(|_, _, cx| {
                                                cx.open_url("https://github.com/trawls-app/trawls")
                                            }),
                                    ),
                                ),
                        )
                        .child(
                            v_flex()
                                .flex_1()
                                .h_full()
                                .overflow_hidden()
                                .gap_0()
                                .child(
                                    h_flex()
                                        .id("header")
                                        .justify_between()
                                        .items_start()
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
                                        .child(div()),
                                )
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

fn sample_lightframes() -> Vec<FrameInfo> {
    vec![
        FrameInfo {
            path: "/mock/lights/frame_0001.ARW",
            filename: "frame_0001.ARW",
            exposure_seconds: 20.0,
            interval_seconds: None,
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T22:00:03",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0002.ARW",
            filename: "frame_0002.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T22:00:24",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0003.ARW",
            filename: "frame_0003.ARW",
            exposure_seconds: 25.0,
            interval_seconds: Some(9.5),
            aperture: 3.2,
            iso: 1600,
            creation_time: "2026-01-11T22:00:58",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0004.ARW",
            filename: "frame_0004.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 2000,
            creation_time: "2026-01-11T22:01:19",
            error: None,
        },
        FrameInfo {
            path: "/mock/lights/frame_0005.ARW",
            filename: "frame_0005.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T22:01:40",
            error: Some("Could not parse EXIF metadata"),
        },
    ]
}

fn sample_darkframes() -> Vec<FrameInfo> {
    vec![
        FrameInfo {
            path: "/mock/darks/dark_001.ARW",
            filename: "dark_001.ARW",
            exposure_seconds: 20.0,
            interval_seconds: None,
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T21:30:11",
            error: None,
        },
        FrameInfo {
            path: "/mock/darks/dark_002.ARW",
            filename: "dark_002.ARW",
            exposure_seconds: 20.0,
            interval_seconds: Some(1.0),
            aperture: 2.8,
            iso: 1600,
            creation_time: "2026-01-11T21:30:32",
            error: None,
        },
    ]
}

fn main() {
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);

        let _ = ThemeRegistry::watch_dir(PathBuf::from("../gpui-component/themes"), cx, |cx| {
            if let Some(theme) = ThemeRegistry::global(cx).themes().get("Ayu Dark").cloned() {
                Theme::global_mut(cx).dark_theme = theme.clone();
                Theme::global_mut(cx).apply_config(&theme);
            }
            Theme::change(ThemeMode::Dark, None, cx);
            cx.refresh_windows();
        });

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(960.0),
                    height: px(680.0),
                }),
                ..Default::default()
            };

            cx.open_window(options, |window, cx| {
                Theme::change(ThemeMode::Dark, Some(window), cx);
                let view = cx.new(|cx| TrawlsDummy::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
