use gpui::App;
use gpui_component::{Theme, ThemeMode, ThemeRegistry};
use std::path::PathBuf;

pub fn configure_theme(cx: &mut App) {
    let _ = ThemeRegistry::watch_dir(PathBuf::from("../gpui-component/themes"), cx, |cx| {
        if let Some(theme) = ThemeRegistry::global(cx).themes().get("Ayu Dark").cloned() {
            Theme::global_mut(cx).dark_theme = theme.clone();
            Theme::global_mut(cx).apply_config(&theme);
        }
        Theme::change(ThemeMode::Dark, None, cx);
        cx.refresh_windows();
    });
}
