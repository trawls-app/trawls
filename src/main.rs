mod processing;
mod ui;

use gpui::{AppContext, Application, WindowOptions, px};
use gpui_component::{Root, Theme, ThemeMode, TitleBar};
use ui::{app::Trawls, assets::Assets};

fn program_description() -> String {
    format!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

fn main() {
    pretty_env_logger::init();
    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);
        cx.activate(true);
        ui::theme::configure_theme(cx);

        cx.spawn(async move |cx| {
            let options = WindowOptions {
                titlebar: Some(TitleBar::title_bar_options()),
                window_min_size: Some(gpui::Size {
                    width: px(960.0),
                    height: px(680.0),
                }),
                app_id: Some("trawls".into()),
                ..Default::default()
            };

            cx.open_window(options, |window, cx| {
                Theme::change(ThemeMode::Dark, Some(window), cx);
                let view = cx.new(|cx| Trawls::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            })?;

            Ok::<_, anyhow::Error>(())
        })
        .detach();
    });
}
