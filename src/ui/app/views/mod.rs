pub(crate) mod image_viewer;
mod shell;
mod steps;
mod table;

fn light_dark(is_light: bool, light: &'static str, dark: &'static str) -> &'static str {
    if is_light { light } else { dark }
}

fn format_file_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(0.0), "0s");
        assert_eq!(format_duration(1.0), "1s");
        assert_eq!(format_duration(45.0), "45s");
        assert_eq!(format_duration(59.4), "59s");
    }

    #[test]
    fn format_duration_minutes() {
        assert_eq!(format_duration(60.0), "1m 00s");
        assert_eq!(format_duration(90.0), "1m 30s");
        assert_eq!(format_duration(3599.0), "59m 59s");
    }

    #[test]
    fn format_duration_hours() {
        assert_eq!(format_duration(3600.0), "1h 00m 00s");
        assert_eq!(format_duration(3661.0), "1h 01m 01s");
        assert_eq!(format_duration(7384.0), "2h 03m 04s");
    }

    #[test]
    fn format_file_size_bytes() {
        assert_eq!(format_file_size(0), "0 B");
        assert_eq!(format_file_size(512), "512 B");
        assert_eq!(format_file_size(1023), "1023 B");
    }

    #[test]
    fn format_file_size_kilobytes() {
        assert_eq!(format_file_size(1024), "1 KB");
        assert_eq!(format_file_size(1536), "2 KB");
    }

    #[test]
    fn format_file_size_megabytes() {
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_file_size(1024 * 1024 + 512 * 1024), "1.5 MB");
        assert_eq!(format_file_size(100 * 1024 * 1024), "100.0 MB");
    }

    #[test]
    fn format_file_size_gigabytes() {
        assert_eq!(format_file_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_file_size(2 * 1024 * 1024 * 1024), "2.0 GB");
    }
}
