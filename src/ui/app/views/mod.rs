mod shell;
mod steps;
mod table;

fn light_dark(is_light: bool, light: &'static str, dark: &'static str) -> &'static str {
    if is_light { light } else { dark }
}
