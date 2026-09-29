//! Starts a Claude Code session on a note, in a new terminal window, and remembers the
//! folders sessions start in, to offer the usual ones first.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

/// A folder a session started in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workdir {
    /// As typed, `~` for the home directory.
    pub path: String,
    /// How many sessions started there.
    pub uses: u32,
    /// When the last one did, in milliseconds since the Unix epoch.
    pub last: u64,
}

/// The most folders remembered; the least used go first, never the one just used.
const REMEMBERED: usize = 30;

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

/// Where memo remembers them: `workdirs.json` beside its settings.
pub fn workdirs_file(settings_file: &Path) -> PathBuf {
    settings_file.with_file_name("workdirs.json")
}

/// The remembered folders; a missing or unreadable file holds none.
pub fn load_workdirs(file: &Path) -> Vec<Workdir> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save_workdirs(file: &Path, workdirs: &[Workdir]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(workdirs).map_err(|e| e.to_string())?;
    std::fs::write(file, text + "\n").map_err(|e| e.to_string())
}

/// Counts a session started in `path` at `now`.
pub fn record(workdirs: &mut Vec<Workdir>, path: &str, now: u64) {
    match workdirs.iter_mut().find(|w| w.path == path) {
        Some(w) => {
            w.uses += 1;
            w.last = now;
        }
        None => workdirs.push(Workdir {
            path: path.to_string(),
            uses: 1,
            last: now,
        }),
    }
    if workdirs.len() > REMEMBERED {
        let kept: Vec<String> = rank(workdirs, now)
            .into_iter()
            .filter(|p| p != path)
            .take(REMEMBERED - 1)
            .collect();
        workdirs.retain(|w| w.path == path || kept.contains(&w.path));
    }
}

/// The folders, most used first, a use counting for less as it ages: in full for a day,
/// half for a week, a quarter for a month, an eighth after.
pub fn rank(workdirs: &[Workdir], now: u64) -> Vec<String> {
    let score = |w: &Workdir| {
        let age = now.saturating_sub(w.last);
        let weight = match age {
            a if a < DAY_MS => 8,
            a if a < 7 * DAY_MS => 4,
            a if a < 30 * DAY_MS => 2,
            _ => 1,
        };
        u64::from(w.uses) * weight
    };
    let mut ranked: Vec<&Workdir> = workdirs.iter().collect();
    ranked.sort_by(|a, b| score(b).cmp(&score(a)).then(b.last.cmp(&a.last)));
    ranked.into_iter().map(|w| w.path.clone()).collect()
}

/// The git repository the note mentions most, else the home directory.
///
/// Documents usually name the files they are about (`~/Code/x/src/main.rs`); each such
/// path is walked up to its repository root, and the most cited root wins.
pub fn guess_workdir(content: &str, home: &Path, ignore: &[PathBuf]) -> PathBuf {
    let mut counts: HashMap<PathBuf, usize> = HashMap::new();
    let mut order = Vec::new();
    for token in content.split(|c: char| c.is_whitespace() || "`'\"()[]<>{},;*|".contains(c)) {
        let token = token.trim_end_matches(['.', ':']);
        let path = if let Some(rest) = token.strip_prefix("~/") {
            home.join(rest)
        } else if token.starts_with('/') && token.len() > 1 {
            PathBuf::from(token)
        } else {
            continue;
        };
        let Some(root) = repository_root(&path) else {
            continue;
        };
        if root == home
            || ignore
                .iter()
                .any(|i| i.starts_with(&root) || root.starts_with(i))
        {
            continue;
        }
        if !counts.contains_key(&root) {
            order.push(root.clone());
        }
        *counts.entry(root).or_default() += 1;
    }
    let best = order
        .into_iter()
        .enumerate()
        .max_by_key(|(i, root)| (counts[root], usize::MAX - i));
    best.map_or_else(|| home.to_path_buf(), |(_, root)| root)
}

/// The closest existing ancestor of `path` holding a `.git`.
fn repository_root(path: &Path) -> Option<PathBuf> {
    let mut dir = path;
    while !dir.exists() {
        dir = dir.parent()?;
    }
    dir.ancestors()
        .find(|d| d.join(".git").exists())
        .map(Path::to_path_buf)
}

/// What Claude is asked, by default, depending on the kind of document.
pub fn default_prompt(path: &Path, folder: &str) -> String {
    let file = path.display();
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if name.contains("handoff") {
        format!("Read the handoff at {file} and continue the work it describes.")
    } else if folder.contains("plan") {
        format!("Read the plan at {file} and carry it out.")
    } else {
        format!("Read {file}; it is the context for what follows.")
    }
}

/// Quotes `text` as a single shell word.
pub fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

/// Ghostty's AppleScript: a new window running the user's own shell, which is typed the
/// `claude` command, so the session gets the usual PATH and the shell stays afterwards.
const GHOSTTY: &str = r#"on run argv
    tell application "Ghostty"
        activate
        set cfg to new surface configuration
        set initial working directory of cfg to item 1 of argv
        set initial input of cfg to item 2 of argv
        new window with configuration cfg
    end tell
end run"#;

const TERMINAL: &str = r#"on run argv
    tell application "Terminal"
        activate
        do script item 2 of argv
    end tell
end run"#;

/// Opens a terminal window in `workdir` running `claude "<prompt>"`: Ghostty when
/// installed, else Terminal. The values travel as osascript arguments, never inside the
/// script, and are shell-quoted for the command line.
pub fn launch(workdir: &Path, prompt: &str) -> Result<(), String> {
    if !workdir.is_dir() {
        return Err(format!("{} is not a directory", workdir.display()));
    }
    let dir = workdir.to_string_lossy();
    let claude = format!("claude {}", shell_quote(prompt));
    let ghostty = Path::new("/Applications/Ghostty.app").exists();
    let (script, input) = if ghostty {
        (GHOSTTY, format!("{claude}\n"))
    } else {
        (TERMINAL, format!("cd {} && {claude}", shell_quote(&dir)))
    };
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", script, "--", &dir, &input])
        .output()
        .map_err(|e| format!("osascript: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_for_the_shell() {
        assert_eq!(shell_quote("it's $HOME"), r"'it'\''s $HOME'");
    }

    #[test]
    fn guesses_the_most_cited_repository() {
        let home = tempfile::tempdir().unwrap();
        let home = home.path().canonicalize().unwrap();
        for repo in ["Code/a", "Code/b"] {
            std::fs::create_dir_all(home.join(repo).join(".git")).unwrap();
        }
        std::fs::create_dir_all(home.join("Code/b/src")).unwrap();
        let notes = home.join("Notes");
        let doc = format!(
            "See `~/Code/a/README.md`, then ~/Code/b/src/main.rs and ~/Code/b/src/new.rs (to create). Also {}.",
            notes.join("x.md").display()
        );
        assert_eq!(guess_workdir(&doc, &home, &[notes]), home.join("Code/b"));
        assert_eq!(guess_workdir("no paths here", &home, &[]), home);
    }

    #[test]
    fn ranks_the_folders_by_uses_as_they_age() {
        let now = 100 * DAY_MS;
        let mut dirs = Vec::new();
        for _ in 0..3 {
            record(&mut dirs, "~/old", now - 60 * DAY_MS);
        }
        record(&mut dirs, "~/new", now);
        record(&mut dirs, "~/week", now - 2 * DAY_MS);
        record(&mut dirs, "~/week", now - 2 * DAY_MS);
        assert_eq!(dirs[0].uses, 3);
        // 3 uses two months ago weigh 3, 1 today 8, 2 this week 8, broken by the latest.
        assert_eq!(rank(&dirs, now), ["~/new", "~/week", "~/old"]);
    }

    #[test]
    fn forgets_the_least_used_folders() {
        let mut dirs = Vec::new();
        for _ in 0..3 {
            record(&mut dirs, "~/kept", 0);
        }
        // Thirty more, each later than the one before: the older ones go first.
        for i in 1..=30 {
            record(&mut dirs, &format!("~/{i}"), i);
        }
        record(&mut dirs, "~/latest", 31);
        assert_eq!(dirs.len(), REMEMBERED);
        let has = |p: &str| dirs.iter().any(|w| w.path == p);
        assert!(has("~/kept") && has("~/latest") && !has("~/1") && !has("~/2") && has("~/3"));
    }

    #[test]
    fn prompts_by_kind() {
        let p = Path::new("/n/plans/26-09-20-x-handoff.md");
        assert!(default_prompt(p, "plans").starts_with("Read the handoff"));
        assert!(default_prompt(Path::new("/n/plans/x.md"), "plans").starts_with("Read the plan"));
        assert!(
            default_prompt(Path::new("/n/reports/x.md"), "reports")
                .starts_with("Read /n/reports/x.md")
        );
    }
}
