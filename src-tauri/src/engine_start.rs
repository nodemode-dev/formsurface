use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const APP: &str = "FormSurface";
pub const SHELL_ENV: &str = "FORMSURFACE_PARENT_PID";
pub const ADDRESS_WAIT: Duration = Duration::from_secs(90);
pub const EXIT_WAIT: Duration = Duration::from_secs(1);
pub const BUSY_CODE: i32 = 2;
const DATA_ENV: &str = "FORMSURFACE_DATA";
const HOME_NAME: &str = "FormSurface";
const LOG_NAME: &str = "formsurface-engine.log";
const KEY_MARK: &str = "window-key";
const TAIL_LINES: usize = 12;
const LINE_CHARS: usize = 300;

#[derive(Debug, PartialEq)]
pub enum Failure {
    Missing,
    NotStarted(String),
    Stopped(Option<i32>),
    TimedOut,
    Window,
}

#[derive(Default)]
pub struct Tail(VecDeque<String>);

impl Tail {
    pub fn push(&mut self, line: &str) {
        if line.contains(KEY_MARK) {
            return;
        }
        let clean: String = line.chars().filter(|c| !c.is_control()).take(LINE_CHARS).collect();
        if clean.trim().is_empty() {
            return;
        }
        if self.0.len() == TAIL_LINES {
            self.0.pop_front();
        }
        self.0.push_back(clean);
    }

    pub fn lines(&self) -> Vec<String> {
        self.0.iter().cloned().collect()
    }
}

pub fn sentence(failure: &Failure) -> String {
    match failure {
        Failure::Missing => format!("{APP} could not start: its engine program is missing from the install. Installing {APP} again puts it back."),
        Failure::NotStarted(reason) => format!("{APP} could not start its engine: {reason}"),
        Failure::Stopped(Some(BUSY_CODE)) => format!("{APP} could not start: another copy of its engine holds its data folder."),
        Failure::Stopped(Some(code)) => format!("{APP} could not start: its engine stopped with code {code} before the window could open."),
        Failure::Stopped(None) => format!("{APP} could not start: its engine stopped before the window could open."),
        Failure::TimedOut => format!("{APP} could not start: it waited {} seconds for its engine to say which address it serves on and that did not happen.", ADDRESS_WAIT.as_secs()),
        Failure::Window => format!("{APP} could not open its window."),
    }
}

pub fn message(failure: &Failure, tail: &[String], log: Option<&Path>) -> String {
    let mut text = sentence(failure);
    if !tail.is_empty() {
        text.push_str("\n\nThe engine's last lines:\n");
        text.push_str(&tail.join("\n"));
    } else if matches!(failure, Failure::Stopped(_) | Failure::TimedOut) {
        text.push_str("\n\nThe engine wrote nothing before that.");
    }
    if let Some(path) = log {
        text.push_str("\n\nThe engine's log: ");
        text.push_str(&path.to_string_lossy());
    }
    text
}

pub fn log_path(var: &dyn Fn(&str) -> Option<String>, home: Option<&Path>) -> Option<PathBuf> {
    let folder = match var(DATA_ENV).filter(|value| !value.is_empty()) {
        Some(raw) => match (raw.strip_prefix('~'), home) {
            (Some(rest), Some(home)) if rest.is_empty() || rest.starts_with('/') || rest.starts_with('\\') => {
                home.join(rest.trim_start_matches(['/', '\\']))
            }
            _ => PathBuf::from(raw),
        },
        None => home?.join(HOME_NAME),
    };
    Some(folder.join("Logs").join(LOG_NAME))
}

pub fn engine_log() -> Option<PathBuf> {
    log_path(&|name: &str| std::env::var(name).ok(), std::env::home_dir().as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tail_keeps_the_last_lines_and_never_the_window_key() {
        let mut tail = Tail::default();
        for n in 0..15 {
            tail.push(&format!("line {n}"));
        }
        tail.push("formsurface-window-key: AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        tail.push("   ");
        tail.push(&"x".repeat(1000));
        let lines = tail.lines();
        assert_eq!(lines.len(), TAIL_LINES);
        assert_eq!(lines[0], "line 4");
        assert_eq!(lines[TAIL_LINES - 1].len(), LINE_CHARS);
        assert!(lines.iter().all(|line| !line.contains("window-key")));
    }

    #[test]
    fn every_failure_has_its_sentence_the_last_lines_and_the_log() {
        let log = Path::new("/data/Logs/formsurface-engine.log");
        let said = vec!["formsurface is already running on /data (engine process 7).".to_string()];
        let busy = message(&Failure::Stopped(Some(BUSY_CODE)), &said, Some(log));
        assert!(busy.starts_with("FormSurface could not start: another copy of its engine holds its data folder."));
        assert!(busy.contains("The engine's last lines:\nformsurface is already running on /data (engine process 7)."));
        assert!(busy.ends_with("The engine's log: /data/Logs/formsurface-engine.log"));
        assert!(message(&Failure::Stopped(Some(3)), &[], Some(log)).contains("stopped with code 3"));
        assert!(message(&Failure::Stopped(None), &[], None).ends_with("The engine wrote nothing before that."));
        assert!(message(&Failure::Missing, &[], None).ends_with("puts it back."));
        assert!(sentence(&Failure::NotStarted("Access is denied.".to_string())).ends_with("its engine: Access is denied."));
        assert!(sentence(&Failure::TimedOut).contains("waited 90 seconds for its engine to say which address it serves on"));
        assert_eq!(sentence(&Failure::Window), "FormSurface could not open its window.");
    }

    #[test]
    fn the_log_is_where_the_engine_writes_it() {
        let home = Path::new("/home/u");
        let none = |_: &str| None;
        let empty = |name: &str| (name == DATA_ENV).then(String::new);
        let set = |name: &str| (name == DATA_ENV).then(|| "~/x".to_string());
        let absolute = |name: &str| (name == DATA_ENV).then(|| "/d/y".to_string());
        let end = Path::new("Logs").join(LOG_NAME);
        assert_eq!(log_path(&none, Some(home)), Some(home.join("FormSurface").join(&end)));
        assert_eq!(log_path(&empty, Some(home)), Some(home.join("FormSurface").join(&end)));
        assert_eq!(log_path(&set, Some(home)), Some(home.join("x").join(&end)));
        assert_eq!(log_path(&absolute, None), Some(Path::new("/d/y").join(&end)));
        assert_eq!(log_path(&none, None), None);
    }
}
