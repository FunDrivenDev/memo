//! Finds the folders Claude Code writes its documents to, from its own configuration.
//!
//! Two sources, read in order:
//! - `plansDirectory` in `settings.json` and `settings.local.json`;
//! - the paths listed in the fenced code blocks of the user's `CLAUDE.md`, one per line,
//!   ending with `/` (e.g. `~/Notes/claude/reports/   what was found or done`).

use std::path::{Path, PathBuf};

use serde::Serialize;

/// A folder of documents, shown as one section of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
}

/// The Claude Code configuration directory: `$CLAUDE_CONFIG_DIR`, or `~/.claude`.
pub fn claude_dir(home: &Path) -> PathBuf {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => home.join(".claude"),
    }
}

/// The existing folders the configuration in `claude_dir` names, without duplicates.
pub fn discover(home: &Path, claude_dir: &Path) -> Vec<Folder> {
    let mut paths = Vec::new();
    for file in ["settings.json", "settings.local.json"] {
        if let Ok(text) = std::fs::read_to_string(claude_dir.join(file)) {
            paths.extend(plans_directory(&text, home));
        }
    }
    if let Ok(text) = std::fs::read_to_string(claude_dir.join("CLAUDE.md")) {
        paths.extend(fenced_directories(&text, home));
    }

    let mut folders: Vec<Folder> = Vec::new();
    for path in paths {
        let Ok(path) = path.canonicalize() else {
            continue;
        };
        if !path.is_dir() || folders.iter().any(|f| f.path == path) {
            continue;
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        folders.push(Folder { name, path });
    }
    disambiguate(&mut folders);
    folders
}

/// The `plansDirectory` of a settings file, resolved against the home directory.
fn plans_directory(settings: &str, home: &Path) -> Option<PathBuf> {
    let value: serde_json::Value = serde_json::from_str(settings).ok()?;
    let dir = value.get("plansDirectory")?.as_str()?;
    Some(expand(dir, home))
}

/// The directories listed at the start of a line inside fenced code blocks.
fn fenced_directories(markdown: &str, home: &Path) -> Vec<PathBuf> {
    let mut inside = false;
    let mut found = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            inside = !inside;
            continue;
        }
        if !inside {
            continue;
        }
        let Some(token) = trimmed.split_whitespace().next() else {
            continue;
        };
        if (token.starts_with("~/") || token.starts_with('/'))
            && token.ends_with('/')
            && token.len() > 2
        {
            found.push(expand(token, home));
        }
    }
    found
}

/// Expands a leading `~`; a relative path is taken from the home directory.
pub fn expand(path: &str, home: &Path) -> PathBuf {
    if path == "~" {
        home.to_path_buf()
    } else if let Some(rest) = path.strip_prefix("~/") {
        home.join(rest)
    } else {
        home.join(path)
    }
}

/// Folders sharing a name get their parent's name in front: `a/plans`, `b/plans`.
fn disambiguate(folders: &mut [Folder]) {
    let names: Vec<String> = folders.iter().map(|f| f.name.clone()).collect();
    for folder in folders.iter_mut() {
        if names.iter().filter(|n| **n == folder.name).count() > 1
            && let Some(parent) = folder.path.parent().and_then(Path::file_name)
        {
            folder.name = format!("{}/{}", parent.to_string_lossy(), folder.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_plans_directory() {
        let home = Path::new("/home/me");
        assert_eq!(
            plans_directory(r#"{"plansDirectory": "~/Notes/plans"}"#, home),
            Some(PathBuf::from("/home/me/Notes/plans"))
        );
        assert_eq!(
            plans_directory(r#"{"plansDirectory": "/abs"}"#, home),
            Some(PathBuf::from("/abs"))
        );
        assert_eq!(plans_directory(r#"{"theme": "auto"}"#, home), None);
        assert_eq!(plans_directory("not json", home), None);
    }

    #[test]
    fn reads_only_fenced_directories() {
        let md = "Keep them out of `~/.claude/plans/`.\n\n```\n~/Notes/claude/plans/     what to do next\n~/Notes/claude/reports/   what was found\nnot/a/path\n~/file.md\n```\n";
        let home = Path::new("/h");
        assert_eq!(
            fenced_directories(md, home),
            vec![
                PathBuf::from("/h/Notes/claude/plans/"),
                PathBuf::from("/h/Notes/claude/reports/")
            ]
        );
    }

    #[test]
    fn discovers_existing_folders_once() {
        let home = tempfile::tempdir().unwrap();
        let claude = home.path().join(".claude");
        std::fs::create_dir_all(home.path().join("n/plans")).unwrap();
        std::fs::create_dir_all(home.path().join("n/reports")).unwrap();
        std::fs::create_dir_all(&claude).unwrap();
        std::fs::write(
            claude.join("settings.json"),
            r#"{"plansDirectory": "~/n/plans"}"#,
        )
        .unwrap();
        std::fs::write(
            claude.join("CLAUDE.md"),
            "```\n~/n/plans/\n~/n/reports/\n~/n/missing/\n```\n",
        )
        .unwrap();

        let folders = discover(home.path(), &claude);
        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["plans", "reports"]);
    }

    #[test]
    fn disambiguates_same_names() {
        let mut folders = vec![
            Folder {
                name: "plans".into(),
                path: "/a/plans".into(),
            },
            Folder {
                name: "plans".into(),
                path: "/b/plans".into(),
            },
            Folder {
                name: "reports".into(),
                path: "/a/reports".into(),
            },
        ];
        disambiguate(&mut folders);
        let names: Vec<&str> = folders.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["a/plans", "b/plans", "reports"]);
    }
}
