use gpui::{
    Context, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex, v_flex,
};

use crate::{
    app::TrawlsDummy,
    model::{FrameInfo, SortKey},
};

impl TrawlsDummy {
    pub(crate) fn render_frame_table(
        &self,
        rows: &[FrameInfo],
        show_interval: bool,
        is_light: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let threshold = Self::interval_warning_threshold(rows).unwrap_or(9999.0);
        let baseline_settings = rows
            .iter()
            .find(|r| r.error.is_none())
            .map(|r| (r.exposure_seconds, r.aperture, r.iso));
        let active_sort = if is_light {
            self.light_sort
        } else {
            self.dark_sort
        };
        let selected_paths = if is_light {
            &self.selected_light_paths
        } else {
            &self.selected_dark_paths
        };
        let visible_paths: Vec<String> = rows.iter().map(|row| row.path.to_string()).collect();
        let selected_visible_count = rows
            .iter()
            .filter(|row| selected_paths.contains(row.path))
            .count();
        let all_visible_selected = !rows.is_empty() && selected_visible_count == rows.len();
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
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .px_2()
                    .py_2()
                    .rounded(cx.theme().radius)
                    .bg(cx.theme().sidebar.alpha(0.35))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Button::new(if is_light {
                                    "select-all-light-rows"
                                } else {
                                    "select-all-dark-rows"
                                })
                                .small()
                                .icon(IconName::Check)
                                .label("Select all")
                                .on_click(cx.listener({
                                    let visible_paths = visible_paths.clone();
                                    move |this, _, _, cx| {
                                        if is_light {
                                            for path in &visible_paths {
                                                this.selected_light_paths.insert(path.clone());
                                            }
                                        } else {
                                            for path in &visible_paths {
                                                this.selected_dark_paths.insert(path.clone());
                                            }
                                        }
                                        cx.notify();
                                    }
                                })),
                            )
                            .child(
                                Button::new(if is_light {
                                    "clear-light-selection"
                                } else {
                                    "clear-dark-selection"
                                })
                                .small()
                                .icon(IconName::Close)
                                .label("Clear selection")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if is_light {
                                            this.selected_light_paths.clear();
                                        } else {
                                            this.selected_dark_paths.clear();
                                        }
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                Button::new(if is_light {
                                    "remove-selected-light"
                                } else {
                                    "remove-selected-dark"
                                })
                                .small()
                                .warning()
                                .icon(IconName::Minus)
                                .label("Remove selected")
                                .disabled(selected_visible_count == 0)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        if is_light {
                                            let selected = this.selected_light_paths.clone();
                                            this.lightframes.retain(|f| !selected.contains(f.path));
                                            this.selected_light_paths.clear();
                                        } else {
                                            let selected = this.selected_dark_paths.clone();
                                            this.darkframes.retain(|f| !selected.contains(f.path));
                                            this.selected_dark_paths.clear();
                                        }
                                        cx.notify();
                                    },
                                )),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} selected", selected_visible_count)),
                    ),
            )
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
                        div().w(px(36.)).child(
                            Checkbox::new(if is_light {
                                "header-select-all-light"
                            } else {
                                "header-select-all-dark"
                            })
                            .checked(all_visible_selected)
                            .on_click(cx.listener({
                                let visible_paths = visible_paths.clone();
                                move |this, checked, _, cx| {
                                    if is_light {
                                        if *checked {
                                            for path in &visible_paths {
                                                this.selected_light_paths.insert(path.clone());
                                            }
                                        } else {
                                            for path in &visible_paths {
                                                this.selected_light_paths.remove(path);
                                            }
                                        }
                                    } else if *checked {
                                        for path in &visible_paths {
                                            this.selected_dark_paths.insert(path.clone());
                                        }
                                    } else {
                                        for path in &visible_paths {
                                            this.selected_dark_paths.remove(path);
                                        }
                                    }
                                    cx.notify();
                                }
                            })),
                        ),
                    )
                    .child(
                        div().w(px(290.)).child(
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
                    ),
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
                                .when(selected_paths.contains(row.path), |this| {
                                    this.bg(cx.theme().primary.alpha(0.18))
                                })
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener({
                                        let path = row.path.to_string();
                                        move |this, _, _, cx| {
                                            if is_light {
                                                if this.selected_light_paths.contains(&path) {
                                                    this.selected_light_paths.remove(&path);
                                                } else {
                                                    this.selected_light_paths.insert(path.clone());
                                                }
                                            } else if this.selected_dark_paths.contains(&path) {
                                                this.selected_dark_paths.remove(&path);
                                            } else {
                                                this.selected_dark_paths.insert(path.clone());
                                            }
                                            cx.notify();
                                        }
                                    }),
                                )
                                .text_xs()
                                .child(
                                    div().w(px(36.)).child(
                                        Checkbox::new(if is_light {
                                            ("select-err-light", ix)
                                        } else {
                                            ("select-err-dark", ix)
                                        })
                                        .checked(selected_paths.contains(row.path))
                                        .on_click(
                                            cx.listener({
                                                let path = row.path.to_string();
                                                move |this, checked, _, cx| {
                                                    if is_light {
                                                        if *checked {
                                                            this.selected_light_paths
                                                                .insert(path.clone());
                                                        } else {
                                                            this.selected_light_paths.remove(&path);
                                                        }
                                                    } else if *checked {
                                                        this.selected_dark_paths
                                                            .insert(path.clone());
                                                    } else {
                                                        this.selected_dark_paths.remove(&path);
                                                    }
                                                    cx.notify();
                                                }
                                            }),
                                        ),
                                    ),
                                )
                                .child(div().w(px(290.)).child(row.filename))
                                .child(div().w(px(570.)).child(error))
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
                            .when(selected_paths.contains(row.path), |this| {
                                this.bg(cx.theme().primary.alpha(0.18))
                            })
                            .cursor_pointer()
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener({
                                    let path = row.path.to_string();
                                    move |this, _, _, cx| {
                                        if is_light {
                                            if this.selected_light_paths.contains(&path) {
                                                this.selected_light_paths.remove(&path);
                                            } else {
                                                this.selected_light_paths.insert(path.clone());
                                            }
                                        } else if this.selected_dark_paths.contains(&path) {
                                            this.selected_dark_paths.remove(&path);
                                        } else {
                                            this.selected_dark_paths.insert(path.clone());
                                        }
                                        cx.notify();
                                    }
                                }),
                            )
                            .text_xs()
                            .child(
                                div().w(px(36.)).child(
                                    Checkbox::new(if is_light {
                                        ("select-light", ix)
                                    } else {
                                        ("select-dark", ix)
                                    })
                                    .checked(selected_paths.contains(row.path))
                                    .on_click(cx.listener({
                                        let path = row.path.to_string();
                                        move |this, checked, _, cx| {
                                            if is_light {
                                                if *checked {
                                                    this.selected_light_paths.insert(path.clone());
                                                } else {
                                                    this.selected_light_paths.remove(&path);
                                                }
                                            } else if *checked {
                                                this.selected_dark_paths.insert(path.clone());
                                            } else {
                                                this.selected_dark_paths.remove(&path);
                                            }
                                            cx.notify();
                                        }
                                    })),
                                ),
                            )
                            .child(div().w(px(290.)).child(row.filename))
                            .child(
                                div()
                                    .w(px(90.))
                                    .text_center()
                                    .when(
                                        baseline_settings
                                            .is_some_and(|(exp, _, _)| row.exposure_seconds != exp),
                                        |this| this.bg(cx.theme().warning.alpha(0.2)),
                                    )
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
                                    .when(
                                        baseline_settings
                                            .is_some_and(|(_, apt, _)| row.aperture != apt),
                                        |this| this.bg(cx.theme().warning.alpha(0.2)),
                                    )
                                    .child(format!("f/{:.1}", row.aperture)),
                            )
                            .child(
                                div()
                                    .w(px(90.))
                                    .text_center()
                                    .when(
                                        baseline_settings.is_some_and(|(_, _, iso)| row.iso != iso),
                                        |this| this.bg(cx.theme().warning.alpha(0.2)),
                                    )
                                    .child(format!("{}", row.iso)),
                            )
                            .child(div().w(px(210.)).text_center().child(row.creation_time))
                            .into_any_element()
                    }))),
            )
    }
}
