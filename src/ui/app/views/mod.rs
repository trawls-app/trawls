mod shell;
mod steps;
mod table;

fn light_dark(is_light: bool, light: &'static str, dark: &'static str) -> &'static str {
    if is_light { light } else { dark }
}

fn format_duration(seconds: f32) -> String {
    let total = seconds.round() as u64;
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}h {m:02}m {s:02}s")
    } else if m > 0 {
        format!("{m}m {s:02}s")
    } else {
        format!("{s}s")
    }
}
