use std::rc::Rc;

use gpui::{
    Context, InteractiveElement as _, IntoElement, MouseButton, ObjectFit, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, StyledImage as _, div, img,
    prelude::FluentBuilder as _, px, size,
};
use gpui_component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    h_flex,
    scroll::{Scrollbar, ScrollbarShow},
    v_flex, v_virtual_list,
};

use crate::ui::{
    app::Trawls,
    model::{FrameInfo, MetadataStatus, SortKey},
};

use super::light_dark;

impl Trawls {
    pub(crate) fn render_frame_table(
        &self,
        rows: Vec<FrameInfo>,
        show_interval: bool,
        is_light: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if rows.is_empty() {
            return v_flex()
                .w_full()
                .flex_1()
                .min_h_0()
                .items_center()
                .justify_center()
                .gap_2()
                .py_12()
                .rounded(cx.theme().radius)
                .border_1()
                .border_color(cx.theme().border)
                .child(
                    Icon::new(IconName::Inbox)
                        .text_color(cx.theme().muted_foreground),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(if is_light {
                            "No lightframes added yet. Click \"Add images\" above to get started."
                        } else {
                            "No darkframes added yet. Click \"Add images\" above to add optional darkframes."
                        }),
                )
                .into_any_element();
        }

        let threshold = Self::interval_warning_threshold(&rows).unwrap_or(9999.0);
        let filename_col_w = rows
            .iter()
            .map(|row| row.filename.len() as f32 * 8.0 + 56.0)
            .fold(290.0_f32, f32::max)
            .clamp(290.0, 820.0);
        let select_col_w = 36.0_f32;
        let thumb_col_w = 64.0_f32;
        let exposure_col_w = 90.0_f32;
        let interval_col_w = if show_interval { 90.0 } else { 0.0 };
        let aperture_col_w = 90.0_f32;
        let iso_col_w = 90.0_f32;
        let time_col_w = 210.0_f32;
        let table_total_w = select_col_w
            + thumb_col_w
            + filename_col_w
            + exposure_col_w
            + interval_col_w
            + aperture_col_w
            + iso_col_w
            + time_col_w;
        let error_col_w = table_total_w - select_col_w - thumb_col_w - filename_col_w;
        let baseline_settings = rows.iter().find_map(|r| {
            if r.metadata_status != MetadataStatus::Loaded || r.error.is_some() {
                return None;
            }
            Some((r.exposure_seconds?, r.aperture?, r.iso?))
        });
        let active_sort = if is_light {
            self.light_sort
        } else {
            self.dark_sort
        };
        let selected_paths = self.selected_paths(is_light);
        let visible_paths: Rc<Vec<String>> = Rc::new(rows.iter().map(|row| row.path.clone()).collect());
        let selected_visible_count = rows
            .iter()
            .filter(|row| selected_paths.contains(row.path.as_str()))
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
        let row_count = rows.len();
        let rows_rc = Rc::new(rows);
        let item_sizes = Rc::new(vec![size(px(table_total_w), px(44.0)); row_count]);

        v_flex()
            .w_full()
            .flex_1()
            .min_h_0()
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
                                Button::new(light_dark(is_light, "select-all-light-rows", "select-all-dark-rows"))
                                .small()
                                .icon(IconName::Check)
                                .label("Select all")
                                .on_click(cx.listener({
                                    let visible_paths = visible_paths.clone();
                                    move |this, _, _, cx| {
                                        for path in visible_paths.iter() {
                                            this.set_selection(path, is_light, true);
                                        }
                                        cx.notify();
                                    }
                                })),
                            )
                            .child(
                                Button::new(light_dark(is_light, "clear-light-selection", "clear-dark-selection"))
                                .small()
                                .icon(IconName::Close)
                                .label("Clear selection")
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.selected_paths_mut(is_light).clear();
                                        cx.notify();
                                    },
                                )),
                            )
                            .child(
                                Button::new(light_dark(is_light, "remove-selected-light", "remove-selected-dark"))
                                .small()
                                .warning()
                                .icon(IconName::Minus)
                                .label("Remove selected")
                                .disabled(selected_visible_count == 0)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| {
                                        this.remove_selected_frames(is_light, cx);
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
            .child(
                h_flex()
                    .min_w(px(table_total_w))
                    .w_full()
                    .px_2()
                    .py_1p5()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().sidebar_accent.alpha(0.7))
                    .font_semibold()
                    .text_xs()
                    .child(
                        div().w(px(select_col_w)).child(
                            Checkbox::new(light_dark(is_light, "header-select-all-light", "header-select-all-dark"))
                            .checked(all_visible_selected)
                            .on_click(cx.listener({
                                let visible_paths = visible_paths.clone();
                                move |this, checked, _, cx| {
                                    for path in visible_paths.iter() {
                                        this.set_selection(path, is_light, *checked);
                                    }
                                    cx.notify();
                                }
                            })),
                        ),
                    )
                    .child(div().w(px(thumb_col_w)))
                    .child(
                        div().w(px(filename_col_w)).child(
                            Button::new(light_dark(is_light, "header-sort-light-filename", "header-sort-dark-filename"))
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
                    .child(div().w(px(exposure_col_w)).text_center().child("Exposure"))
                    .when(show_interval, |this| {
                        this.child(div().w(px(interval_col_w)).text_center().child("Interval"))
                    })
                    .child(div().w(px(aperture_col_w)).text_center().child("Aperture"))
                    .child(div().w(px(iso_col_w)).text_center().child("ISO"))
                    .child(
                        div().w(px(time_col_w)).child(
                            Button::new(light_dark(is_light, "header-sort-light-time", "header-sort-dark-time"))
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
                div()
                    .relative()
                    .flex_1()
                    .min_h_0()
                    .child(
                        v_virtual_list(
                            cx.entity().clone(),
                            light_dark(is_light, "frame-table-virtual-light", "frame-table-virtual-dark"),
                            item_sizes,
                            move |this, visible_range, _, cx| {
                                visible_range
                                    .map(|ix| {
                                        let row = &rows_rc[ix];
                                        let selected_paths = this.selected_paths(is_light);
                                        let is_selected = selected_paths.contains(row.path.as_str());
                                        let has_error = row.error.is_some();

                                        let base = h_flex()
                                            .min_w(px(table_total_w))
                                            .w_full()
                                            .px_2()
                                            .py_1()
                                            .border_b_1()
                                            .border_color(cx.theme().border)
                                            .when(has_error, |el| {
                                                el.bg(cx.theme().danger.alpha(0.18))
                                            })
                                            .when(!has_error && ix % 2 == 1, |el| {
                                                el.bg(cx.theme().sidebar_accent.alpha(0.35))
                                            })
                                            .when(is_selected, |el| {
                                                el.bg(cx.theme().primary.alpha(0.18))
                                            })
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener({
                                                    let path = row.path.to_string();
                                                    move |this, _, _, cx| {
                                                        this.toggle_selection(&path, is_light);
                                                        cx.notify();
                                                    }
                                                }),
                                            )
                                            .text_xs()
                                            .child(
                                                div().w(px(select_col_w)).child(
                                                    Checkbox::new(if is_light {
                                                        ("select-light", ix)
                                                    } else {
                                                        ("select-dark", ix)
                                                    })
                                                    .checked(is_selected)
                                                    .on_click(cx.listener({
                                                        let path = row.path.to_string();
                                                        move |this, checked, _, cx| {
                                                            this.set_selection(&path, is_light, *checked);
                                                            cx.notify();
                                                        }
                                                    })),
                                                ),
                                            )
                                            .child(
                                                div()
                                                    .id(("thumb-cell", ix))
                                                    .w(px(thumb_col_w))
                                                    .flex()
                                                    .items_center()
                                                    .justify_center()
                                                    .cursor_pointer()
                                                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                                        cx.stop_propagation();
                                                    })
                                                    .on_click(cx.listener({
                                                        let path = row.path.to_string();
                                                        move |this, _, window, cx| {
                                                            this.inspect_frame(path.clone(), window, cx);
                                                        }
                                                    }))
                                                    .child(if let Some(thumb) = &row.thumbnail {
                                                        img(thumb.clone())
                                                            .w(px(56.0))
                                                            .h(px(38.0))
                                                            .rounded(px(2.0))
                                                            .object_fit(ObjectFit::Cover)
                                                            .into_any_element()
                                                    } else {
                                                        div()
                                                            .w(px(56.0))
                                                            .h(px(38.0))
                                                            .rounded(px(2.0))
                                                            .bg(cx.theme().sidebar_accent.alpha(0.3))
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .when(
                                                                row.metadata_status == MetadataStatus::Pending,
                                                                |el| el.child(
                                                                    Icon::new(IconName::LoaderCircle)
                                                                        .xsmall()
                                                                        .text_color(cx.theme().muted_foreground),
                                                                ),
                                                            )
                                                            .into_any_element()
                                                    }),
                                            )
                                            .child(
                                                div().w(px(filename_col_w)).child(
                                                    h_flex()
                                                        .gap_2()
                                                        .items_center()
                                                        .child(match row.metadata_status {
                                                            MetadataStatus::Pending => {
                                                                Icon::new(IconName::LoaderCircle)
                                                                    .xsmall()
                                                                    .text_color(
                                                                        cx.theme().muted_foreground,
                                                                    )
                                                                    .into_any_element()
                                                            }
                                                            MetadataStatus::Loaded => {
                                                                Icon::new(IconName::CircleCheck)
                                                                    .xsmall()
                                                                    .text_color(cx.theme().success)
                                                                    .into_any_element()
                                                            }
                                                            MetadataStatus::Failed => {
                                                                Icon::new(IconName::TriangleAlert)
                                                                    .xsmall()
                                                                    .text_color(cx.theme().danger)
                                                                    .into_any_element()
                                                            }
                                                        })
                                                        .child(row.filename.clone()),
                                                ),
                                            );

                                        if let Some(error) = &row.error {
                                            return base
                                                .child(
                                                    div().w(px(error_col_w)).child(error.clone()),
                                                )
                                                .into_any_element();
                                        }

                                        base
                                            .child(
                                                div()
                                                    .w(px(exposure_col_w))
                                                    .text_center()
                                                    .when(
                                                        baseline_settings.is_some_and(
                                                            |(exp, _, _)| {
                                                                row.exposure_seconds.is_some_and(
                                                                    |value| value != exp,
                                                                )
                                                            },
                                                        ),
                                                        |this| {
                                                            this.bg(cx.theme().warning.alpha(0.2))
                                                        },
                                                    )
                                                    .child(match row.exposure_seconds {
                                                        Some(value) => format!("{value:.1}s"),
                                                        None => "...".to_string(),
                                                    }),
                                            )
                                            .when(show_interval, |this| {
                                                this.child(
                                                    div()
                                                        .w(px(interval_col_w))
                                                        .text_center()
                                                        .when(
                                                            row.interval_seconds.is_some_and(
                                                                |value| value.abs() > threshold,
                                                            ),
                                                            |this| {
                                                                this.bg(cx
                                                                    .theme()
                                                                    .warning
                                                                    .alpha(0.2))
                                                            },
                                                        )
                                                        .child(match row.interval_seconds {
                                                            Some(value) => format!("{value:.1}s"),
                                                            None => "...".to_string(),
                                                        }),
                                                )
                                            })
                                            .child(
                                                div()
                                                    .w(px(aperture_col_w))
                                                    .text_center()
                                                    .when(
                                                        baseline_settings.is_some_and(
                                                            |(_, apt, _)| {
                                                                row.aperture.is_some_and(|value| {
                                                                    value != apt
                                                                })
                                                            },
                                                        ),
                                                        |this| {
                                                            this.bg(cx.theme().warning.alpha(0.2))
                                                        },
                                                    )
                                                    .child(match row.aperture {
                                                        Some(value) => format!("f/{value:.1}"),
                                                        None => "...".to_string(),
                                                    }),
                                            )
                                            .child(
                                                div()
                                                    .w(px(iso_col_w))
                                                    .text_center()
                                                    .when(
                                                        baseline_settings.is_some_and(
                                                            |(_, _, iso)| {
                                                                row.iso.is_some_and(|value| {
                                                                    value != iso
                                                                })
                                                            },
                                                        ),
                                                        |this| {
                                                            this.bg(cx.theme().warning.alpha(0.2))
                                                        },
                                                    )
                                                    .child(match row.iso {
                                                        Some(value) => value.to_string(),
                                                        None => "...".to_string(),
                                                    }),
                                            )
                                            .child(
                                                div().w(px(time_col_w)).text_center().child(
                                                    row.creation_time
                                                        .map(|dt| {
                                                            dt.format("%Y-%m-%dT%H:%M:%S")
                                                                .to_string()
                                                        })
                                                        .unwrap_or_else(|| "...".to_string()),
                                                ),
                                            )
                                            .into_any_element()
                                    })
                                    .collect::<Vec<_>>()
                            },
                        )
                        .track_scroll(&self.table_scroll_handle),
                    )
                    .child(
                        div().absolute().top_0().right_0().bottom_0().child(
                            Scrollbar::vertical(
                                &self.table_scroll_state,
                                &self.table_scroll_handle,
                            )
                            .scrollbar_show(ScrollbarShow::Always),
                        ),
                    ),
            )
            .into_any_element()
    }
}
