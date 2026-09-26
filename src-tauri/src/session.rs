//! Starts a Claude Code session on a note, in a new terminal window.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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
