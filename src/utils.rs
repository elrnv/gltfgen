use indicatif::{ProgressBar, ProgressStyle};
use log;
use regex::Regex;

#[macro_export]
macro_rules! log {
    ($msgs:ident; $($arg:tt)*) => ({
        let msg = format!($($arg)*);
        if let Some((count, last_msg)) = $msgs.last_mut() {
            if last_msg == &msg {
                *count += 1;
            } else {
                $msgs.push((1, msg));
            }
        } else {
            $msgs.push((1, msg));
        }
    });
}

enum MessageType {
    Warn,
    Info,
}

pub fn print_warnings(messages: Vec<(usize, String)>) {
    print_messages(messages, MessageType::Warn);
}
pub fn print_info(messages: Vec<(usize, String)>) {
    print_messages(messages, MessageType::Info);
}

fn print_messages(messages: Vec<(usize, String)>, msg_type: MessageType) {
    for (count, warning) in messages {
        let msg = if count > 1 {
            format!("({}) {}", count, warning)
        } else {
            format!("{}", warning)
        };
        match msg_type {
            MessageType::Warn => log::warn!("{}", msg),
            MessageType::Info => log::info!("{}", msg),
        }
    }
}

pub fn glob_to_regex(glob: &str) -> Regex {
    let mut regex = String::from("^");

    let mut prev_c = None;
    let mut glob_iter = glob.chars().peekable();
    while let Some(c) = glob_iter.next() {
        match c {
            '#' => {
                // Special character indicating a frame number digit
                regex.push_str("(?P<frame>[0-9]+)");
            }
            // Escape special characters
            '$' | '^' | '+' | '.' | '(' | ')' | '=' | '!' | '|' => {
                regex.push('\\');
                regex.push(c);
            }
            '{' => regex.push('('),
            '}' => regex.push(')'),
            '?' => regex.push('.'),
            '*' => {
                // Check if there are multiple consecutive ** in the pattern.
                let mut count = 1;
                while glob_iter.peek() == Some(&'*') {
                    count += 1;
                    glob_iter.next();
                }
                let next_c = glob_iter.peek();

                if count > 1
                    && (next_c == Some(&'/') || next_c.is_none())
                    && (prev_c == Some('/') || prev_c.is_none())
                {
                    // Multiple * detected
                    // match zero or more path segments
                    regex.push_str("?:[^/]*?:/|$*");
                    glob_iter.next(); // consume '/' if any.
                } else {
                    // Single * detected
                    regex.push_str("[^/]*"); // match one path segment
                }
            }
            _ => regex.push(c),
        }
        prev_c = Some(c);
    }

    regex.push('$');

    Regex::new(&regex).expect("ERROR: Failed to convert glob to regular expression")
}

/// Remove braces from the pattern.
pub fn remove_braces(pattern: &str) -> String {
    let mut out_pattern = String::new();
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' && (chars.peek() == Some(&'{') || chars.peek() == Some(&'}')) {
            out_pattern.push(chars.next().unwrap());
            continue;
        }
        if c == '{' || c == '}' {
            continue;
        }

        out_pattern.push(c);
    }
    out_pattern
}

pub fn new_progress_bar(quiet: bool, len: usize) -> ProgressBar {
    if !quiet {
        ProgressBar::new(len as u64).with_style(
            ProgressStyle::default_bar()
                .progress_chars("=> ")
                .template("{elapsed:4} [{bar:20.cyan/blue}] {pos:>7}/{len:7} {msg}")
                .expect("Failed to render the progress bar."),
        )
    } else {
        ProgressBar::hidden()
    }
}

pub fn new_progress_bar_file(quiet: bool, num_bytes: usize) -> ProgressBar {
    if !quiet {
        ProgressBar::new(num_bytes as u64).with_style(
            ProgressStyle::default_bar()
                .progress_chars("=> ")
                .template("{elapsed:4} [{bar:20.green}] {percent:>7}%        {msg}")
                .expect("Failed to render the progress bar."),
        )
    } else {
        ProgressBar::hidden()
    }
}

pub fn new_spinner(quiet: bool) -> ProgressBar {
    let spinner = if !quiet {
        ProgressBar::new_spinner().with_style(
            ProgressStyle::default_spinner()
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")
                .template("{elapsed:4} {spinner:1.cyan/blue} {prefix:32.cyan/blue}     {wide_msg}")
                .expect("Failed to render the progress bar."),
        )
    } else {
        ProgressBar::hidden()
    };
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));
    spinner
}
