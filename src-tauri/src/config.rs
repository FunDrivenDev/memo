//! Finds the folders memo shows.
//!
//! By default, the one folder Claude Code writes its plans to: `plansDirectory` in
//! `settings.local.json`, else in `settings.json`, else `<claude dir>/plans`. The settings
//! window can replace that list with folders of the user's choice, kept per installation
//! in memo's own settings file.
//!
//! Archived notes go to one archive folder, into a subfolder named after the folder they
//! come from. memo sets it on first launch to an `archive` sibling of Claude Code's plans
//! folder; it stays there when that folder moves.

use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A folder of documents, shown as one section of the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
}

/// memo's own settings, stored as JSON in its application support folder.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    /// The folders to show, as typed (`~` allowed); `None` follows Claude Code's plans folder.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folders: Option<Vec<String>>,
    /// The archive folder, as typed; set on first launch by [`pin_archive`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<String>,
}

/// The Claude Code configuration directory: `$CLAUDE_CONFIG_DIR`, or `~/.claude`.
pub fn claude_dir(home: &Path) -> PathBuf {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => home.join(".claude"),
    }
}

/// Where memo keeps its settings: `~/Library/Application Support/dev.rlvdx.memo/settings.json`.
pub fn settings_file(home: &Path) -> PathBuf {
    home.join("Library/Application Support/dev.rlvdx.memo/settings.json")
}

/// The saved settings; missing or unreadable ones are the defaults.
pub fn load(file: &Path) -> Settings {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(file: &Path, settings: &Settings) -> Result<(), String> {
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    std::fs::write(file, text + "\n").map_err(|e| e.to_string())
}

/// Checks a typed folder is absolute or under `~`, and drops its trailing slashes.
pub fn clean_folder(folder: &str) -> Result<String, String> {
    let folder = folder.trim();
    let folder = match folder.trim_end_matches('/') {
        "" if folder.starts_with('/') => "/",
        trimmed => trimmed,
    };
    if folder.is_empty() {
        Err("No folder given".to_string())
    } else if folder == "~" || folder.starts_with("~/") || folder.starts_with('/') {
        Ok(folder.to_string())
    } else {
        Err(format!("{folder} is not an absolute path or one under ~"))
    }
}

/// Trims the folders, drops blank and repeated ones, and refuses any that is not absolute
/// or under `~`.
pub fn clean_folders(folders: &[String]) -> Result<Vec<String>, String> {
    let mut clean: Vec<String> = Vec::new();
    for folder in folders {
        let folder = folder.trim();
        if folder.is_empty() || clean.iter().any(|f| f == folder) {
            continue;
        }
        clean.push(clean_folder(folder)?);
    }
    Ok(clean)
}

/// Claude Code's plans folder, written with `~` when it is under the home directory.
pub fn default_folders(home: &Path, claude_dir: &Path) -> Vec<String> {
    let configured = ["settings.local.json", "settings.json"]
        .iter()
        .find_map(|file| plans_directory(&std::fs::read_to_string(claude_dir.join(file)).ok()?));
    let folder = configured.unwrap_or_else(|| tilde(&claude_dir.join("plans"), home));
    vec![folder]
}

/// The `archive` sibling of Claude Code's plans folder.
pub fn default_archive(home: &Path, claude_dir: &Path) -> String {
    let plans = expand(&default_folders(home, claude_dir)[0], home);
    tilde(&plans.parent().unwrap_or(&plans).join("archive"), home)
}

/// Writes the default archive into the settings when they name none, so that it stays put
/// when Claude Code's plans folder moves.
pub fn pin_archive(file: &Path, home: &Path, claude_dir: &Path) -> Result<(), String> {
    let mut settings = load(file);
    if settings.archive.is_some() {
        return Ok(());
    }
    settings.archive = Some(default_archive(home, claude_dir));
    save(file, &settings)
}

/// The archive folder the settings name, or the default one if they could not be saved.
pub fn archive(settings: &Settings, home: &Path, claude_dir: &Path) -> String {
    settings
        .archive
        .clone()
        .unwrap_or_else(|| default_archive(home, claude_dir))
}

/// Refuses an archive that is a watched folder or holds one directly: archiving would
/// then move notes into the folder they come from, or mix both lists.
pub fn check_archive(archive: &str, folders: &[Folder], home: &Path) -> Result<(), String> {
    let path = expand(archive, home);
    let path = path.canonicalize().unwrap_or(path);
    for folder in folders {
        if folder.path == path || folder.path.parent() == Some(path.as_path()) {
            return Err(format!(
                "The archive {archive} cannot be, or directly hold, the watched folder {}",
                folder.path.display()
            ));
        }
    }
    Ok(())
}

/// The folders the settings name, or Claude Code's plans folder by default.
pub fn folders(settings: &Settings, home: &Path, claude_dir: &Path) -> Vec<String> {
    settings
        .folders
        .clone()
        .unwrap_or_else(|| default_folders(home, claude_dir))
}

/// Whether a folder, as typed, exists.
pub fn exists(folder: &str, home: &Path) -> bool {
    expand(folder, home).is_dir()
}

/// Creates a folder, as typed, with its missing parents.
pub fn create(folder: &str, home: &Path) -> Result<(), String> {
    let path = expand(folder, home);
    std::fs::create_dir_all(&path).map_err(|e| format!("Could not create {folder}: {e}"))
}

/// The existing folders among `folders`, without duplicates.
pub fn discover(folders: &[String], home: &Path) -> Vec<Folder> {
    let mut found: Vec<Folder> = Vec::new();
    for folder in folders {
        let Ok(path) = expand(folder, home).canonicalize() else {
            continue;
        };
        if !path.is_dir() || found.iter().any(|f| f.path == path) {
            continue;
        }
        let name = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        found.push(Folder { name, path });
    }
    disambiguate(&mut found);
    found
}

/// The most folders [`complete`] offers.
const COMPLETIONS: usize = 50;

/// The folders completing a typed path.
#[derive(Debug, Default, PartialEq, Eq, Serialize)]
pub struct Completion {
    /// The subfolders whose name starts with the last component, written as typed.
    pub folders: Vec<String>,
    /// The folder, as typed, macOS refused to list.
    pub denied: Option<String>,
}

/// Lists the subfolders completing the last component of `typed`, case-insensitively;
/// hidden ones only when that component starts with a dot.
pub fn complete(typed: &str, home: &Path) -> Completion {
    let typed = typed.trim_start();
    if typed == "~" {
        return Completion {
            folders: vec!["~/".to_string()],
            denied: None,
        };
    }
    let Some((dir, prefix)) = typed.rsplit_once('/') else {
        return Completion::default();
    };
    if !(dir.is_empty() || dir == "~" || dir.starts_with("~/") || dir.starts_with('/')) {
        return Completion::default();
    }
    let path = if dir.is_empty() {
        PathBuf::from("/")
    } else {
        expand(dir, home)
    };
    let entries = match std::fs::read_dir(&path) {
        Ok(entries) => entries,
        Err(e) if e.kind() == ErrorKind::PermissionDenied => {
            return Completion {
                folders: Vec::new(),
                denied: Some(format!("{dir}/")),
            };
        }
        Err(_) => return Completion::default(),
    };
    let lower = prefix.to_lowercase();
    let mut folders: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let hidden = name.starts_with('.') && !prefix.starts_with('.');
            (!hidden && name.to_lowercase().starts_with(&lower) && entry.path().is_dir())
                .then(|| format!("{dir}/{name}"))
        })
        .collect();
    folders.sort_by_key(|f| f.to_lowercase());
    folders.truncate(COMPLETIONS);
    Completion {
        folders,
        denied: None,
    }
}

/// The `plansDirectory` of a settings file, as written.
fn plans_directory(settings: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(settings).ok()?;
    let dir = value.get("plansDirectory")?.as_str()?.trim();
    (!dir.is_empty()).then(|| dir.to_string())
}

/// Writes a path under the home directory as `~/...`.
pub fn tilde(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if rest.as_os_str().is_empty() => "~".to_string(),
        Ok(rest) => format!("~/{}", rest.display()),
        Err(_) => path.display().to_string(),
    }
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
        assert_eq!(
            plans_directory(r#"{"plansDirectory": "~/Notes/plans"}"#),
            Some("~/Notes/plans".to_string())
        );
        assert_eq!(plans_directory(r#"{"theme": "auto"}"#), None);
        assert_eq!(plans_directory("not json"), None);
    }

    #[test]
    fn defaults_to_the_claude_plans_folder() {
        let home = tempfile::tempdir().unwrap();
        let claude = home.path().join(".claude");
        std::fs::create_dir_all(&claude).unwrap();
        assert_eq!(default_folders(home.path(), &claude), ["~/.claude/plans"]);

        std::fs::write(claude.join("settings.json"), r#"{"plansDirectory": "~/a"}"#).unwrap();
        assert_eq!(default_folders(home.path(), &claude), ["~/a"]);

        std::fs::write(
            claude.join("settings.local.json"),
            r#"{"plansDirectory": "/b"}"#,
        )
        .unwrap();
        assert_eq!(default_folders(home.path(), &claude), ["/b"]);
    }

    #[test]
    fn custom_folders_replace_the_default() {
        let settings = Settings {
            folders: Some(vec!["~/x".into()]),
            ..Settings::default()
        };
        let home = Path::new("/h");
        assert_eq!(folders(&settings, home, &home.join(".claude")), ["~/x"]);
    }

    #[test]
    fn archive_defaults_beside_the_plans_folder() {
        let home = tempfile::tempdir().unwrap();
        let claude = home.path().join(".claude");
        std::fs::create_dir_all(&claude).unwrap();
        assert_eq!(default_archive(home.path(), &claude), "~/.claude/archive");
        std::fs::write(
            claude.join("settings.json"),
            r#"{"plansDirectory": "~/Notes/claude/plans"}"#,
        )
        .unwrap();
        assert_eq!(
            default_archive(home.path(), &claude),
            "~/Notes/claude/archive"
        );
        let settings = Settings {
            archive: Some("/elsewhere".into()),
            ..Settings::default()
        };
        assert_eq!(archive(&settings, home.path(), &claude), "/elsewhere");
    }

    #[test]
    fn pins_the_archive_on_first_launch() {
        let home = tempfile::tempdir().unwrap();
        let claude = home.path().join(".claude");
        let file = home.path().join("memo/settings.json");
        std::fs::create_dir_all(&claude).unwrap();
        pin_archive(&file, home.path(), &claude).unwrap();
        assert_eq!(load(&file).archive.as_deref(), Some("~/.claude/archive"));

        std::fs::write(
            claude.join("settings.json"),
            r#"{"plansDirectory": "~/n/plans"}"#,
        )
        .unwrap();
        pin_archive(&file, home.path(), &claude).unwrap();
        assert_eq!(load(&file).archive.as_deref(), Some("~/.claude/archive"));
    }

    #[test]
    fn completes_folders() {
        let home = tempfile::tempdir().unwrap();
        for dir in [
            "Notes/plans",
            "Notes/reports",
            "notebook",
            ".hidden",
            "Nope",
        ] {
            std::fs::create_dir_all(home.path().join(dir)).unwrap();
        }
        std::fs::write(home.path().join("Notes/plan.md"), "").unwrap();
        let folders = |typed| complete(typed, home.path()).folders;

        assert_eq!(folders("~/no"), ["~/Nope", "~/notebook", "~/Notes"]);
        assert_eq!(folders("~/Notes/"), ["~/Notes/plans", "~/Notes/reports"]);
        assert_eq!(folders("~/Notes/p"), ["~/Notes/plans"]);
        assert_eq!(folders("~/.h"), ["~/.hidden"]);
        assert_eq!(folders("~"), ["~/"]);
        assert!(folders("relative/p").is_empty());
        assert!(folders("~/missing/").is_empty());
        let absolute = format!("{}/Notes/r", home.path().display());
        assert_eq!(
            complete(&absolute, home.path()).folders,
            [format!("{}/Notes/reports", home.path().display())]
        );
    }

    #[test]
    fn refuses_an_archive_over_the_folders() {
        let home = Path::new("/h");
        let folders = [Folder {
            name: "plans".into(),
            path: "/h/n/plans".into(),
        }];
        assert!(check_archive("~/n/archive", &folders, home).is_ok());
        assert!(check_archive("~/n/plans", &folders, home).is_err());
        assert!(check_archive("~/n", &folders, home).is_err());
    }

    #[test]
    fn creates_folders_with_their_parents() {
        let home = tempfile::tempdir().unwrap();
        assert!(!exists("~/a/b", home.path()));
        create("~/a/b", home.path()).unwrap();
        assert!(exists("~/a/b", home.path()));
        create("~/a/b", home.path()).unwrap();
    }

    #[test]
    fn cleans_folders() {
        let typed = ["  ~/a ", "", "~/a", "/b/", "~/"].map(String::from);
        assert_eq!(clean_folders(&typed).unwrap(), ["~/a", "/b", "~"]);
        assert_eq!(clean_folder("/").unwrap(), "/");
        assert!(clean_folder(" ").is_err());
        assert!(clean_folders(&["relative/path".to_string()]).is_err());
    }

    #[test]
    fn settings_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("memo/settings.json");
        assert_eq!(load(&file), Settings::default());
        let settings = Settings {
            folders: Some(vec!["~/n".into()]),
            archive: Some("~/a".into()),
        };
        save(&file, &settings).unwrap();
        assert_eq!(load(&file), settings);
    }

    #[test]
    fn discovers_existing_folders_once() {
        let home = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(home.path().join("n/plans")).unwrap();
        std::fs::create_dir_all(home.path().join("n/reports")).unwrap();
        let typed = ["~/n/plans", "~/n/plans/", "~/n/reports", "~/n/missing"].map(String::from);

        let folders = discover(&typed, home.path());
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
