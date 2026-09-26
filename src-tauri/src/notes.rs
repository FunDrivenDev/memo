//! The Markdown documents of the folders: listing, reading, archiving and trashing.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::config::Folder;

/// What the sidebar shows of a document.
#[derive(Debug, Clone, Serialize)]
pub struct Note {
    /// The absolute path, which also identifies the note.
    pub id: String,
    pub folder: String,
    pub file_name: String,
    pub title: String,
    pub excerpt: String,
    /// Last modification, in milliseconds since the Unix epoch.
    pub modified: u64,
    /// The whole file, kept for the search rather than sent to the front end.
    #[serde(skip)]
    pub content: String,
}

/// The Markdown files directly inside each folder, most recently modified first.
pub fn list(folders: &[Folder]) -> Vec<Note> {
    let mut notes = Vec::new();
    for folder in folders {
        for path in markdown_files(&folder.path) {
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let stem = path.file_stem().unwrap_or_default().to_string_lossy();
            notes.push(Note {
                id: path.to_string_lossy().into_owned(),
                folder: folder.name.clone(),
                title: title(&content, &stem),
                excerpt: excerpt(&content),
                modified: modified_ms(&path),
                file_name,
                content,
            });
        }
    }
    notes.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.title.cmp(&b.title))
    });
    notes
}

/// The visible `.md` files directly inside `dir`.
pub fn markdown_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))
                && !p
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .collect()
}

fn modified_ms(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

/// The body of a document, without its YAML front matter.
pub fn body(content: &str) -> &str {
    let Some(rest) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    else {
        return content;
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        offset += line.len();
        if line.trim_end() == "---" {
            return &rest[offset..];
        }
    }
    content
}

/// The first level-1 heading, else the front matter's `title:`, else the file name made
/// readable: a lower heading (`## TL;DR`) says less than the file name.
pub fn title(content: &str, stem: &str) -> String {
    let mut in_fence = false;
    for line in body(content).lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence
            && let Some(text) = t.strip_prefix("# ")
            && !text.trim().is_empty()
        {
            return clean_inline(text.trim().trim_end_matches('#').trim());
        }
    }
    if let Some(front) = content.strip_prefix("---\n") {
        for line in front.lines().take_while(|l| l.trim_end() != "---") {
            if let Some(value) = line.strip_prefix("title:") {
                let value = value.trim().trim_matches(['"', '\'']);
                if !value.is_empty() {
                    return value.to_string();
                }
            }
        }
    }
    readable_stem(stem)
}

/// `26-09-20-vps-bootstrap` → `Vps bootstrap`.
fn readable_stem(stem: &str) -> String {
    let parts: Vec<&str> = stem.split(['-', '_']).collect();
    let skip = parts
        .iter()
        .take(3)
        .take_while(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
        .count();
    let words = parts[skip..].join(" ");
    let mut chars = words.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => stem.to_string(),
    }
}

/// The first paragraph of prose, flattened and cut to about 160 characters.
pub fn excerpt(content: &str) -> String {
    let mut in_fence = false;
    let mut paragraph = String::new();
    for line in body(content).lines() {
        let t = line.trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        let skip = in_fence
            || t.starts_with('#')
            || t.starts_with('|')
            || t.starts_with("<!--")
            || t.starts_with("---")
            || t.starts_with("***");
        if t.is_empty() || skip {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        let t = t.trim_start_matches(['>', '-', '*', '+', ' ']);
        if !paragraph.is_empty() {
            paragraph.push(' ');
        }
        paragraph.push_str(t);
        if paragraph.len() > 400 {
            break;
        }
    }
    let flat = clean_inline(&paragraph);
    truncate(&flat, 160)
}

/// Drops the Markdown emphasis and code marks and keeps link texts.
fn clean_inline(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' | '_' | '`' => {}
            '[' => {}
            ']' if chars.peek() == Some(&'(') => {
                for c in chars.by_ref() {
                    if c == ')' {
                        break;
                    }
                }
            }
            ']' => {}
            _ => out.push(c),
        }
    }
    out
}

pub fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let cut: String = text.chars().take(max).collect();
    let cut = cut.rsplit_once(' ').map_or(cut.as_str(), |(head, _)| head);
    format!("{}…", cut.trim_end_matches([',', ';', ':', '.', ' ']))
}

/// The path of the note `id` once checked to be a Markdown file directly inside one of
/// the folders; anything else is refused, so the front end cannot reach other files.
pub fn resolve(folders: &[Folder], id: &str) -> Result<PathBuf, String> {
    let path = Path::new(id)
        .canonicalize()
        .map_err(|e| format!("{id}: {e}"))?;
    let inside = path
        .parent()
        .is_some_and(|parent| folders.iter().any(|f| f.path == parent));
    let markdown = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("md"));
    if inside && markdown && path.is_file() {
        Ok(path)
    } else {
        Err(format!("{id} is not a document of the configured folders"))
    }
}

/// Where the archived notes of `folder` go: an `archive/<folder>` sibling, so the folders
/// themselves stay flat.
pub fn archive_dir(folder: &Path) -> PathBuf {
    let name = folder.file_name().unwrap_or_default();
    folder.parent().unwrap_or(folder).join("archive").join(name)
}

/// Moves the note to its folder's archive, never overwriting an archived note.
pub fn archive(path: &Path) -> Result<PathBuf, String> {
    let folder = path.parent().ok_or("no parent folder")?;
    let dir = archive_dir(folder);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    let mut target = dir.join(path.file_name().unwrap_or_default());
    let mut n = 1;
    while target.exists() {
        n += 1;
        target = dir.join(format!("{stem}-{n}.{ext}"));
    }
    std::fs::rename(path, &target).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(target)
}

/// Moves the note to the macOS Trash, from where Finder can put it back.
pub fn trash(path: &Path) -> Result<(), String> {
    trash::delete(path).map_err(|e| format!("{}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_prefers_the_first_h1_then_the_file_name() {
        assert_eq!(title("intro\n\n# The **plan**\n\n## Next", "x"), "The plan");
        assert_eq!(
            title("```\n# not a title\n```\n# Real one", "x"),
            "Real one"
        );
        assert_eq!(
            title("---\ntitle: \"From front\"\n---\nbody", "x"),
            "From front"
        );
        assert_eq!(
            title("#hashtag only", "26-09-20-vps-bootstrap-first-run"),
            "Vps bootstrap first run"
        );
        assert_eq!(title("", "notes"), "Notes");
    }

    #[test]
    fn excerpt_takes_the_first_paragraph() {
        let md = "---\nx: y\n---\n# Title\n\n> **Goal**: ship the `app`\n> fast, see [docs](http://x).\n\nSecond.";
        assert_eq!(excerpt(md), "Goal: ship the app fast, see docs.");
        let long = format!("# T\n\n{}", "word ".repeat(80));
        assert!(excerpt(&long).ends_with('…'));
        assert!(excerpt(&long).chars().count() <= 161);
    }

    #[test]
    fn body_strips_front_matter() {
        assert_eq!(body("---\na: 1\n---\nhello"), "hello");
        assert_eq!(body("---\nunclosed"), "---\nunclosed");
        assert_eq!(body("plain"), "plain");
    }

    fn setup() -> (tempfile::TempDir, Vec<Folder>) {
        let root = tempfile::tempdir().unwrap();
        let plans = root.path().join("plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::write(plans.join("a.md"), "# A").unwrap();
        std::fs::write(plans.join(".hidden.md"), "# H").unwrap();
        std::fs::write(plans.join("b.txt"), "B").unwrap();
        std::fs::write(root.path().join("outside.md"), "# O").unwrap();
        let folders = vec![Folder {
            name: "plans".into(),
            path: plans.canonicalize().unwrap(),
        }];
        (root, folders)
    }

    #[test]
    fn lists_only_visible_markdown() {
        let (_root, folders) = setup();
        let notes = list(&folders);
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].title, "A");
        assert_eq!(notes[0].folder, "plans");
    }

    #[test]
    fn resolve_refuses_files_outside_the_folders() {
        let (root, folders) = setup();
        let plans = &folders[0].path;
        assert!(resolve(&folders, &plans.join("a.md").to_string_lossy()).is_ok());
        assert!(resolve(&folders, &plans.join("b.txt").to_string_lossy()).is_err());
        assert!(resolve(&folders, &root.path().join("outside.md").to_string_lossy()).is_err());
        assert!(resolve(&folders, &plans.join("../outside.md").to_string_lossy()).is_err());
        assert!(resolve(&folders, "/etc/hosts").is_err());
    }

    #[test]
    fn archive_never_overwrites() {
        let (root, folders) = setup();
        let plans = &folders[0].path;
        let first = archive(&plans.join("a.md")).unwrap();
        std::fs::write(plans.join("a.md"), "# A again").unwrap();
        let second = archive(&plans.join("a.md")).unwrap();
        let dir = root.path().canonicalize().unwrap().join("archive/plans");
        assert_eq!(first, dir.join("a.md"));
        assert_eq!(second, dir.join("a-2.md"));
        assert!(list(&folders).is_empty());
    }
}
