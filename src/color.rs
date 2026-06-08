use std::sync::atomic::{AtomicBool, Ordering};

static COLOR_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn init_color() {
    #[cfg(windows)]
    {
        let enabled = ansi_supported();
        COLOR_ENABLED.store(enabled, Ordering::SeqCst);
    }
    #[cfg(not(windows))]
    {
        COLOR_ENABLED.store(true, Ordering::SeqCst);
    }
}

pub fn set_color_enabled(enabled: bool) {
    COLOR_ENABLED.store(enabled, Ordering::SeqCst);
}

pub fn color_enabled() -> bool {
    COLOR_ENABLED.load(Ordering::SeqCst)
}

#[cfg(windows)]
fn ansi_supported() -> bool {
    use std::env;
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if env::var_os("CLICOLOR_FORCE").is_some() {
        return true;
    }
    true
}

#[cfg(not(windows))]
fn ansi_supported() -> bool {
    use std::env;
    if env::var_os("NO_COLOR").is_some() {
        return false;
    }
    if env::var_os("CLICOLOR_FORCE").is_some() {
        return true;
    }
    true
}

pub fn red(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[31m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn green(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[32m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn yellow(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[33m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn blue(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[34m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn magenta(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[35m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn cyan(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[36m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn white(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[37m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn bold(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[1m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn dimmed(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[2m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn red_bold(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[1;31m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn green_bold(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[1;32m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn yellow_bold(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[1;33m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}

pub fn magenta_bold(s: &str) -> String {
    if color_enabled() {
        format!("\x1b[1;35m{}\x1b[0m", s)
    } else {
        s.to_string()
    }
}
