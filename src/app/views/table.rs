use gpui::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _,
    StatefulInteractiveElement as _, Styled as _, div, prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme as _, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
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
}
