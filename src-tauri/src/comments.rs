//! Comments on notes: free text attached to a passage or to blocks of a note, like the
//! comments of a pull request review. The note stays read-only: they are kept in memo's
//! own data, beside its settings, and follow a note when memo archives or restores it.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// What a comment is attached to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Anchor {
    /// `text` for a selected passage, `block` for whole paragraphs, list items, code or tables.
    pub kind: String,
    /// The source lines it spans when it was written, to find it again after an edit.
    pub start: u32,
    pub end: u32,
    /// The text it covers, whitespace collapsed.
    pub quote: String,
    /// A few characters before and after a passage, whitespace removed, telling repeats apart.
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    /// The note's path.
    pub note: String,
    pub body: String,
    pub anchor: Anchor,
    /// Milliseconds since the Unix epoch.
    pub created: u64,
    pub updated: u64,
}

/// Where memo keeps the comments: `comments.json` beside its settings.
pub fn file(settings_file: &Path) -> PathBuf {
    settings_file.with_file_name("comments.json")
}

/// The saved comments; a missing or unreadable file holds none.
pub fn load(file: &Path) -> Vec<Comment> {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Writes the comments through a temporary file, so a crash never leaves half of them.
pub fn save(file: &Path, comments: &[Comment]) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(comments).map_err(|e| e.to_string())?;
    let partial = file.with_extension("json.partial");
    std::fs::write(&partial, text + "\n").map_err(|e| e.to_string())?;
    std::fs::rename(&partial, file).map_err(|e| e.to_string())
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

fn clean_body(body: &str) -> Result<String, String> {
    let body = body.trim();
    if body.is_empty() {
        Err("A comment needs some text".to_string())
    } else {
        Ok(body.to_string())
    }
}

/// Adds a comment and returns its id, unique among the comments.
pub fn add(
    comments: &mut Vec<Comment>,
    note: String,
    body: &str,
    anchor: Anchor,
    now: u64,
) -> Result<String, String> {
    let body = clean_body(body)?;
    if anchor.kind != "text" && anchor.kind != "block" {
        return Err(format!("Unknown anchor {}", anchor.kind));
    }
    let mut id = format!("{now:x}");
    let mut n = 1;
    while comments.iter().any(|c| c.id == id) {
        n += 1;
        id = format!("{now:x}-{n}");
    }
    comments.push(Comment {
        id: id.clone(),
        note,
        body,
        anchor,
        created: now,
        updated: now,
    });
    Ok(id)
}

pub fn edit(comments: &mut [Comment], id: &str, body: &str, now: u64) -> Result<(), String> {
    let body = clean_body(body)?;
    let comment = comments
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or("That comment is gone")?;
    if comment.body != body {
        comment.body = body;
        comment.updated = now;
    }
    Ok(())
}

/// Removes a comment, returning it.
pub fn remove(comments: &mut Vec<Comment>, id: &str) -> Result<Comment, String> {
    let i = comments
        .iter()
        .position(|c| c.id == id)
        .ok_or("That comment is gone")?;
    Ok(comments.remove(i))
}

/// Puts back a removed comment, unless it is there already.
pub fn restore(comments: &mut Vec<Comment>, comment: Comment) {
    if !comments.iter().any(|c| c.id == comment.id) {
        comments.push(comment);
    }
}

/// Moves the comments of a note to its new path; true if any moved.
pub fn rename(comments: &mut [Comment], from: &Path, to: &Path) -> bool {
    let (from, to) = (from.to_string_lossy(), to.to_string_lossy());
    let mut moved = false;
    for comment in comments.iter_mut().filter(|c| c.note == from) {
        comment.note = to.to_string();
        moved = true;
    }
    moved
}

/// Drops the comments of a note gone to the Trash; true if it had any.
pub fn forget(comments: &mut Vec<Comment>, note: &Path) -> bool {
    let note = note.to_string_lossy();
    let before = comments.len();
    comments.retain(|c| c.note != note);
    comments.len() != before
}

#[cfg(test)]
mod tests {
    use super::*;

    fn anchor() -> Anchor {
        Anchor {
            kind: "text".into(),
            start: 3,
            end: 3,
            quote: "some words".into(),
            prefix: String::new(),
            suffix: String::new(),
        }
    }

    #[test]
    fn adds_edits_and_removes() {
        let mut comments = Vec::new();
        let id = add(
            &mut comments,
            "/n/a.md".into(),
            "  check this ",
            anchor(),
            10,
        )
        .unwrap();
        assert_eq!(comments[0].body, "check this");
        let again = add(&mut comments, "/n/a.md".into(), "and this", anchor(), 10).unwrap();
        assert_ne!(id, again);

        edit(&mut comments, &id, "checked", 20).unwrap();
        assert_eq!(
            (comments[0].body.as_str(), comments[0].updated),
            ("checked", 20)
        );
        assert!(edit(&mut comments, &id, " ", 30).is_err());

        let removed = remove(&mut comments, &id).unwrap();
        assert_eq!(comments.len(), 1);
        restore(&mut comments, removed.clone());
        restore(&mut comments, removed);
        assert_eq!(comments.len(), 2);
        assert!(remove(&mut comments, "missing").is_err());
    }

    #[test]
    fn refuses_empty_comments_and_unknown_anchors() {
        let mut comments = Vec::new();
        assert!(add(&mut comments, "/n/a.md".into(), "\n ", anchor(), 1).is_err());
        let odd = Anchor {
            kind: "page".into(),
            ..anchor()
        };
        assert!(add(&mut comments, "/n/a.md".into(), "x", odd, 1).is_err());
    }

    #[test]
    fn follows_the_note_and_forgets_it_when_trashed() {
        let mut comments = Vec::new();
        add(&mut comments, "/n/a.md".into(), "one", anchor(), 1).unwrap();
        add(&mut comments, "/n/b.md".into(), "two", anchor(), 2).unwrap();
        assert!(rename(
            &mut comments,
            Path::new("/n/a.md"),
            Path::new("/arch/n/a.md")
        ));
        assert_eq!(comments[0].note, "/arch/n/a.md");
        assert!(!rename(
            &mut comments,
            Path::new("/x.md"),
            Path::new("/y.md")
        ));
        assert!(forget(&mut comments, Path::new("/n/b.md")));
        assert_eq!(comments.len(), 1);
    }

    #[test]
    fn round_trips_through_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = file(&dir.path().join("memo/settings.json"));
        assert!(load(&file).is_empty());
        let mut comments = Vec::new();
        add(&mut comments, "/n/a.md".into(), "one", anchor(), 1).unwrap();
        save(&file, &comments).unwrap();
        assert_eq!(load(&file), comments);
    }
}
