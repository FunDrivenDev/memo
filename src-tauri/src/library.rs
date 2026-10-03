//! The library: the watched folders and the archive, their notes, the comments on them,
//! and the last move to undo.
//!
//! Every rule about which note may go where lives here, so the commands only adapt it to
//! the front end. A note is named by its path, checked against the folders before
//! anything touches the disk.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::comments::{self, Anchor, Comment};
use crate::config::Folder;
use crate::notes::{self, Note};
use crate::search;

pub struct Library {
    comments_file: PathBuf,
    comments: Vec<Comment>,
    folders: Vec<Folder>,
    notes: Vec<Note>,
    /// The archive folder, which may not exist yet.
    archive: PathBuf,
    /// The sections of the archive, and their notes.
    archive_folders: Vec<Folder>,
    archived: Vec<Note>,
    /// The last archive or restore, which [`Library::undo`] moves back.
    last_move: Option<Move>,
}

struct Move {
    from: PathBuf,
    to: PathBuf,
}

/// What the front end shows of the library.
#[derive(Serialize)]
pub struct Snapshot {
    folders: Vec<Folder>,
    notes: Vec<Note>,
    archive: PathBuf,
    archive_folders: Vec<Folder>,
    archived: Vec<Note>,
    comments: Vec<Comment>,
}

impl Library {
    /// An empty library, with the comments saved in `comments_file`.
    pub fn new(comments_file: PathBuf) -> Library {
        Library {
            comments: comments::load(&comments_file),
            comments_file,
            folders: Vec::new(),
            notes: Vec::new(),
            archive: PathBuf::new(),
            archive_folders: Vec::new(),
            archived: Vec::new(),
            last_move: None,
        }
    }

    /// Rereads the notes of `folders` and of the archive.
    pub fn scan(&mut self, folders: Vec<Folder>, archive: &Path) {
        self.archive = archive
            .canonicalize()
            .unwrap_or_else(|_| archive.to_path_buf());
        self.archive_folders = notes::archive_folders(&self.archive);
        self.notes = notes::list(&folders);
        self.archived = notes::list(&self.archive_folders);
        self.folders = folders;
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            folders: self.folders.clone(),
            notes: self.notes.clone(),
            archive: self.archive.clone(),
            archive_folders: self.archive_folders.clone(),
            archived: self.archived.clone(),
            comments: self.comments.clone(),
        }
    }

    pub fn folders(&self) -> &[Folder] {
        &self.folders
    }

    /// The folders whose changes the front end should hear of: the watched ones, the
    /// sections of the archive, and the archive itself once it exists.
    pub fn watched(&self) -> Vec<PathBuf> {
        let mut watched: Vec<PathBuf> = self.folders.iter().map(|f| f.path.clone()).collect();
        watched.extend(self.archive_folders.iter().map(|f| f.path.clone()));
        if self.archive.is_dir() {
            watched.push(self.archive.clone());
        }
        watched
    }

    /// The path of a note of the folders or of the archive.
    pub fn resolve(&self, id: &str) -> Result<PathBuf, String> {
        let mut folders = self.folders.clone();
        folders.extend(self.archive_folders.iter().cloned());
        notes::resolve(&folders, id)
    }

    pub fn search(&self, query: &str, archived: bool) -> search::Results {
        let notes = if archived {
            &self.archived
        } else {
            &self.notes
        };
        search::search_all(query, notes, &self.comments, 50)
    }

    /// Moves a note of the folders to `<archive>/<folder name>`, returning its new path.
    pub fn archive(&mut self, id: &str) -> Result<PathBuf, String> {
        let path = self.resolve(id)?;
        let folder = path.parent().ok_or("no parent folder")?;
        if !self.folders.iter().any(|f| f.path == folder) {
            return Err(format!("{id} is already archived"));
        }
        let dir = notes::archive_target(&self.archive, folder);
        self.move_note(path, &dir)
    }

    /// Moves an archived note back to the watched folder its archive section is named
    /// after, returning its new path.
    pub fn restore(&mut self, id: &str) -> Result<PathBuf, String> {
        let path = self.resolve(id)?;
        let section = path.parent().and_then(Path::file_name).unwrap_or_default();
        let folder = self
            .folders
            .iter()
            .find(|f| f.path.file_name() == Some(section))
            .map(|f| f.path.clone())
            .ok_or_else(|| {
                format!(
                    "No watched folder is named {}; move the note in Finder",
                    section.to_string_lossy()
                )
            })?;
        self.move_note(path, &folder)
    }

    /// Moves the last archived or restored note back where it was, returning that path.
    pub fn undo(&mut self) -> Result<PathBuf, String> {
        let last = self.last_move.take().ok_or("Nothing to undo")?;
        if !last.to.is_file() {
            return Err(format!("{} is gone", last.to.display()));
        }
        if last.from.exists() {
            return Err(format!("{} exists again", last.from.display()));
        }
        notes::move_file(&last.to, &last.from)?;
        self.follow_move(&last.to, &last.from);
        Ok(last.from)
    }

    /// Throws a note away with `bin`, the macOS Trash outside the tests, and forgets its
    /// comments.
    pub fn trash(
        &mut self,
        id: &str,
        bin: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<(), String> {
        let path = self.resolve(id)?;
        bin(&path)?;
        let _ = self.change_comments(|c| {
            comments::forget(c, &path);
            Ok(())
        });
        Ok(())
    }

    /// Comments a note, on the passage or blocks `anchor` names; returns all the comments.
    pub fn add_comment(
        &mut self,
        note: &str,
        body: &str,
        anchor: Anchor,
    ) -> Result<Vec<Comment>, String> {
        let note = self.resolve(note)?.to_string_lossy().into_owned();
        self.change_comments(|c| comments::add(c, note, body, anchor, comments::now()))
    }

    pub fn edit_comment(&mut self, id: &str, body: &str) -> Result<Vec<Comment>, String> {
        self.change_comments(|c| comments::edit(c, id, body, comments::now()))
    }

    /// Deletes a comment; the front end keeps it to undo that.
    pub fn resolve_comment(&mut self, id: &str) -> Result<Vec<Comment>, String> {
        self.change_comments(|c| comments::remove(c, id))
    }

    /// Puts back a resolved comment, if its note is still in the library.
    pub fn restore_comment(&mut self, comment: Comment) -> Result<Vec<Comment>, String> {
        self.resolve(&comment.note)?;
        self.change_comments(|c| {
            comments::restore(c, comment);
            Ok(())
        })
    }

    /// Changes the comments and saves them, returning them all; on any error they stay
    /// as they were.
    fn change_comments<T>(
        &mut self,
        change: impl FnOnce(&mut Vec<Comment>) -> Result<T, String>,
    ) -> Result<Vec<Comment>, String> {
        let mut next = self.comments.clone();
        change(&mut next)?;
        comments::save(&self.comments_file, &next)?;
        self.comments = next;
        Ok(self.comments.clone())
    }

    /// Carries the comments of a note memo moved to its new path.
    fn follow_move(&mut self, from: &Path, to: &Path) {
        let _ = self.change_comments(|c| {
            comments::rename(c, from, to);
            Ok(())
        });
    }

    /// Moves a note into `dir` and remembers it for `undo`.
    fn move_note(&mut self, from: PathBuf, dir: &Path) -> Result<PathBuf, String> {
        if from.parent() == Some(dir) {
            return Err(format!(
                "{} is already in {}",
                from.display(),
                dir.display()
            ));
        }
        let to = notes::move_into(&from, dir)?;
        self.follow_move(&from, &to);
        self.last_move = Some(Move {
            from,
            to: to.clone(),
        });
        Ok(to)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A library watching `<root>/plans`, holding `a.md`, with its archive in
    /// `<root>/archive`, not created yet, and its comments in `<root>/memo`.
    fn setup() -> (tempfile::TempDir, Library) {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let plans = base.join("plans");
        std::fs::create_dir_all(&plans).unwrap();
        std::fs::write(plans.join("a.md"), "# A").unwrap();
        let mut library = Library::new(base.join("memo/comments.json"));
        library.scan(
            vec![Folder {
                name: "plans".into(),
                path: plans,
            }],
            &base.join("archive"),
        );
        (root, library)
    }

    fn base(root: &tempfile::TempDir) -> PathBuf {
        root.path().canonicalize().unwrap()
    }

    /// Rereads the folders, as the watcher has the front end do after a change.
    fn rescan(library: &mut Library) {
        let archive = library.archive.clone();
        library.scan(library.folders.clone(), &archive);
    }

    fn anchor() -> Anchor {
        Anchor {
            kind: "text".into(),
            start: 1,
            end: 1,
            quote: "A".into(),
            prefix: String::new(),
            suffix: String::new(),
        }
    }

    fn id(path: &Path) -> String {
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn archive_then_undo_puts_the_note_and_its_comments_back() {
        let (root, mut library) = setup();
        let note = base(&root).join("plans/a.md");
        library.add_comment(&id(&note), "check", anchor()).unwrap();

        let archived = library.archive(&id(&note)).unwrap();
        assert_eq!(archived, base(&root).join("archive/plans/a.md"));
        assert!(archived.is_file() && !note.exists());
        assert_eq!(library.snapshot().comments[0].note, id(&archived));
        rescan(&mut library);
        assert_eq!(library.snapshot().archived.len(), 1);

        assert_eq!(library.undo().unwrap(), note);
        assert!(note.is_file() && !archived.exists());
        assert_eq!(library.snapshot().comments[0].note, id(&note));
        assert_eq!(comments::load(&library.comments_file), library.comments);
        assert_eq!(library.undo().unwrap_err(), "Nothing to undo");
    }

    #[test]
    fn restore_goes_back_to_the_folder_of_the_same_name() {
        let (root, mut library) = setup();
        let archived = library
            .archive(&id(&base(&root).join("plans/a.md")))
            .unwrap();
        rescan(&mut library);
        assert_eq!(
            library.restore(&id(&archived)).unwrap(),
            base(&root).join("plans/a.md")
        );
    }

    #[test]
    fn restore_without_a_watched_folder_of_the_same_name_fails() {
        let (root, mut library) = setup();
        let stray = base(&root).join("archive/drafts");
        std::fs::create_dir_all(&stray).unwrap();
        std::fs::write(stray.join("b.md"), "# B").unwrap();
        rescan(&mut library);
        let error = library.restore(&id(&stray.join("b.md"))).unwrap_err();
        assert!(
            error.starts_with("No watched folder is named drafts"),
            "{error}"
        );
        assert!(stray.join("b.md").is_file());
    }

    #[test]
    fn archive_takes_only_notes_of_the_watched_folders() {
        let (root, mut library) = setup();
        let archived = library
            .archive(&id(&base(&root).join("plans/a.md")))
            .unwrap();
        rescan(&mut library);
        let error = library.archive(&id(&archived)).unwrap_err();
        assert!(error.ends_with("is already archived"), "{error}");

        std::fs::write(base(&root).join("outside.md"), "# O").unwrap();
        assert!(
            library
                .archive(&id(&base(&root).join("outside.md")))
                .is_err()
        );
        assert!(base(&root).join("outside.md").is_file());
    }

    #[test]
    fn undo_refuses_a_note_moved_since() {
        let (root, mut library) = setup();
        let archived = library
            .archive(&id(&base(&root).join("plans/a.md")))
            .unwrap();
        std::fs::rename(&archived, base(&root).join("elsewhere.md")).unwrap();
        let error = library.undo().unwrap_err();
        assert!(error.ends_with("is gone"), "{error}");

        std::fs::write(base(&root).join("plans/a.md"), "# A").unwrap();
        library
            .archive(&id(&base(&root).join("plans/a.md")))
            .unwrap();
        std::fs::write(base(&root).join("plans/a.md"), "# A again").unwrap();
        let error = library.undo().unwrap_err();
        assert!(error.ends_with("exists again"), "{error}");
    }

    #[test]
    fn trash_forgets_the_comments_of_the_note() {
        let (root, mut library) = setup();
        let note = base(&root).join("plans/a.md");
        library.add_comment(&id(&note), "check", anchor()).unwrap();
        library
            .trash(&id(&note), |p| {
                std::fs::remove_file(p).map_err(|e| e.to_string())
            })
            .unwrap();
        assert!(!note.exists());
        assert!(library.snapshot().comments.is_empty());
        assert!(comments::load(&library.comments_file).is_empty());
    }

    #[test]
    fn comments_go_only_on_notes_of_the_library() {
        let (root, mut library) = setup();
        std::fs::write(base(&root).join("outside.md"), "# O").unwrap();
        let outside = id(&base(&root).join("outside.md"));
        assert!(library.add_comment(&outside, "no", anchor()).is_err());

        let note = id(&base(&root).join("plans/a.md"));
        let added = library.add_comment(&note, "yes", anchor()).unwrap();
        let resolved = added[0].clone();
        assert!(library.resolve_comment(&resolved.id).unwrap().is_empty());
        assert_eq!(library.restore_comment(resolved).unwrap(), added);
        let edited = library.edit_comment(&added[0].id, "sure").unwrap();
        assert_eq!(edited[0].body, "sure");
    }
}
