//! memo: a keyboard-driven reader for the plans, handoffs and reports Claude Code writes.
//!
//! The front end only ever names a note by its path; every command checks that path
//! against the configured folders before touching the disk.

mod config;
mod notes;
mod render;
mod search;
mod session;

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::Duration;

use notify_debouncer_mini::notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{Debouncer, new_debouncer};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::{AppHandle, Emitter, Manager, State};

use config::Folder;
use notes::Note;

struct App {
    home: PathBuf,
    settings_file: PathBuf,
    folders: Mutex<Vec<Folder>>,
    notes: Mutex<Vec<Note>>,
    /// The archive folder, which may not exist yet.
    archive: Mutex<PathBuf>,
    /// The sections of the archive, and their notes.
    archive_folders: Mutex<Vec<Folder>>,
    archived: Mutex<Vec<Note>>,
    /// The last archive or restore, which `undo` moves back.
    last_move: Mutex<Option<Move>>,
    watched: Mutex<Vec<PathBuf>>,
    watcher: Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

struct Move {
    from: PathBuf,
    to: PathBuf,
}

impl App {
    /// Rereads the settings, the folders and the archive; watches them when they changed.
    fn reload(&self, handle: &AppHandle) -> Library {
        let settings = config::load(&self.settings_file);
        let claude_dir = config::claude_dir(&self.home);
        let folders = config::discover(
            &config::folders(&settings, &self.home, &claude_dir),
            &self.home,
        );
        let archive = config::expand(
            &config::archive(&settings, &self.home, &claude_dir),
            &self.home,
        );
        let archive = archive.canonicalize().unwrap_or(archive);
        let archive_folders = notes::archive_folders(&archive);
        let notes = notes::list(&folders);
        let archived = notes::list(&archive_folders);

        let mut watched: Vec<PathBuf> = folders.iter().map(|f| f.path.clone()).collect();
        watched.extend(archive_folders.iter().map(|f| f.path.clone()));
        if archive.is_dir() {
            watched.push(archive.clone());
        }
        let changed = *self.watched.lock().unwrap() != watched;
        if changed || self.watcher.lock().unwrap().is_none() {
            *self.watcher.lock().unwrap() = watch(handle, &watched);
            *self.watched.lock().unwrap() = watched;
        }
        *self.folders.lock().unwrap() = folders.clone();
        *self.notes.lock().unwrap() = notes.clone();
        *self.archive.lock().unwrap() = archive.clone();
        *self.archive_folders.lock().unwrap() = archive_folders.clone();
        *self.archived.lock().unwrap() = archived.clone();
        Library {
            claude_dir,
            custom: settings.folders.is_some(),
            folders,
            notes,
            archive,
            archive_folders,
            archived,
        }
    }

    /// The path of a note of the folders or of the archive.
    fn resolve(&self, id: &str) -> Result<PathBuf, String> {
        let mut folders = self.folders.lock().unwrap().clone();
        folders.extend(self.archive_folders.lock().unwrap().iter().cloned());
        notes::resolve(&folders, id)
    }

    /// Moves a note and remembers it for `undo`.
    fn move_note(&self, from: PathBuf, dir: &Path) -> Result<PathBuf, String> {
        if from.parent() == Some(dir) {
            return Err(format!(
                "{} is already in {}",
                from.display(),
                dir.display()
            ));
        }
        let to = notes::move_into(&from, dir)?;
        *self.last_move.lock().unwrap() = Some(Move {
            from,
            to: to.clone(),
        });
        Ok(to)
    }
}

/// Emits `library-changed` when a file of the watched folders changes.
fn watch(handle: &AppHandle, folders: &[PathBuf]) -> Option<Debouncer<RecommendedWatcher>> {
    let handle = handle.clone();
    let mut debouncer = new_debouncer(
        Duration::from_millis(300),
        move |result: notify_debouncer_mini::DebounceEventResult| {
            if result.is_ok() {
                let _ = handle.emit("library-changed", ());
            }
        },
    )
    .ok()?;
    for folder in folders {
        let _ = debouncer
            .watcher()
            .watch(folder, RecursiveMode::NonRecursive);
    }
    Some(debouncer)
}

#[derive(Serialize)]
struct Library {
    claude_dir: PathBuf,
    /// Whether the folders come from memo's settings rather than Claude Code's default.
    custom: bool,
    folders: Vec<Folder>,
    notes: Vec<Note>,
    archive: PathBuf,
    archive_folders: Vec<Folder>,
    archived: Vec<Note>,
}

/// What an archive, restore or undo did, with the library after it.
#[derive(Serialize)]
struct Moved {
    /// The note's new path.
    id: PathBuf,
    library: Library,
}

#[derive(Serialize)]
struct SettingsView {
    /// The folders shown, as typed.
    folders: Vec<FolderSetting>,
    /// Claude Code's plans folder, used when no folder is chosen.
    defaults: Vec<String>,
    custom: bool,
    /// The archive folder, as typed.
    archive: String,
    file: PathBuf,
}

#[derive(Serialize)]
struct FolderSetting {
    path: String,
    exists: bool,
}

#[derive(Serialize)]
struct Rendered {
    html: String,
    words: usize,
}

#[derive(Serialize)]
struct SessionDefaults {
    workdir: PathBuf,
    prompt: String,
}

#[tauri::command]
fn library(app: State<App>, handle: AppHandle) -> Library {
    app.reload(&handle)
}

#[tauri::command]
fn settings(app: State<App>) -> SettingsView {
    let settings = config::load(&app.settings_file);
    let claude_dir = config::claude_dir(&app.home);
    let folders = config::folders(&settings, &app.home, &claude_dir)
        .into_iter()
        .map(|path| FolderSetting {
            exists: config::exists(&path, &app.home),
            path,
        })
        .collect();
    SettingsView {
        folders,
        defaults: config::default_folders(&app.home, &claude_dir),
        custom: settings.folders.is_some(),
        archive: config::archive(&settings, &app.home, &claude_dir),
        file: app.settings_file.clone(),
    }
}

/// Saves the folders to show, `None` following Claude Code's plans folder, and the archive.
#[tauri::command]
fn save_settings(
    app: State<App>,
    handle: AppHandle,
    folders: Option<Vec<String>>,
    archive: &str,
) -> Result<Library, String> {
    let settings = config::Settings {
        folders: folders.map(|f| config::clean_folders(&f)).transpose()?,
        archive: Some(config::clean_folder(archive).map_err(|e| format!("Archive: {e}"))?),
    };
    let claude_dir = config::claude_dir(&app.home);
    let watched = config::discover(
        &config::folders(&settings, &app.home, &claude_dir),
        &app.home,
    );
    config::check_archive(
        &config::archive(&settings, &app.home, &claude_dir),
        &watched,
        &app.home,
    )?;
    config::save(&app.settings_file, &settings)?;
    Ok(app.reload(&handle))
}

/// Asks for a folder with the macOS picker; `None` when cancelled.
#[tauri::command]
async fn choose_folder(app: State<'_, App>) -> Result<Option<String>, String> {
    let output = Command::new("/usr/bin/osascript")
        .args(["-e", "POSIX path of (choose folder)"])
        .output()
        .map_err(|e| format!("osascript: {e}"))?;
    if !output.status.success() {
        // Cancelling is error -128.
        let error = String::from_utf8_lossy(&output.stderr);
        return if error.contains("-128") {
            Ok(None)
        } else {
            Err(error.trim().to_string())
        };
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let path = path
        .strip_suffix('/')
        .filter(|p| !p.is_empty())
        .unwrap_or(&path);
    Ok(Some(config::tilde(Path::new(path), &app.home)))
}

#[tauri::command]
fn complete_folder(app: State<App>, typed: &str) -> config::Completion {
    config::complete(typed, &app.home)
}

/// Opens the pane of System Settings that lets memo into a folder: Files and Folders for
/// the ones macOS asks about (Desktop, Documents, Downloads, iCloud Drive), else Full Disk
/// Access.
#[tauri::command]
fn open_privacy_settings(app: State<App>, folder: &str) -> Result<(), String> {
    let path = config::expand(folder, &app.home);
    let asked = [
        "Desktop",
        "Documents",
        "Downloads",
        "Library/Mobile Documents",
    ];
    let pane = if asked.iter().any(|f| path.starts_with(app.home.join(f))) {
        "Privacy_FilesAndFolders"
    } else {
        "Privacy_AllFiles"
    };
    let url = format!("x-apple.systempreferences:com.apple.preference.security?{pane}");
    open(&[OsStr::new(&url)])
}

#[tauri::command]
fn render(app: State<App>, id: &str) -> Result<Rendered, String> {
    let path = app.resolve(id)?;
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let words = notes::body(&content).split_whitespace().count();
    Ok(Rendered {
        html: render::to_html(&content),
        words,
    })
}

#[tauri::command]
fn search(app: State<App>, query: &str, archived: bool) -> Vec<search::Hit> {
    let notes = if archived { &app.archived } else { &app.notes };
    search::search(query, &notes.lock().unwrap(), 50)
}

/// Moves a note of the folders to `<archive>/<folder name>`.
#[tauri::command]
fn archive(app: State<App>, handle: AppHandle, id: &str) -> Result<Moved, String> {
    let path = app.resolve(id)?;
    let folder = path.parent().ok_or("no parent folder")?;
    if !app.folders.lock().unwrap().iter().any(|f| f.path == folder) {
        return Err(format!("{id} is already archived"));
    }
    let dir = notes::archive_target(&app.archive.lock().unwrap(), folder);
    let to = app.move_note(path, &dir)?;
    Ok(Moved {
        id: to,
        library: app.reload(&handle),
    })
}

/// Moves an archived note back to the watched folder its archive section is named after.
#[tauri::command]
fn restore(app: State<App>, handle: AppHandle, id: &str) -> Result<Moved, String> {
    let path = app.resolve(id)?;
    let section = path.parent().and_then(Path::file_name).unwrap_or_default();
    let folder = app
        .folders
        .lock()
        .unwrap()
        .iter()
        .find(|f| f.path.file_name() == Some(section))
        .map(|f| f.path.clone())
        .ok_or_else(|| {
            format!(
                "No watched folder is named {}; move the note in Finder",
                section.to_string_lossy()
            )
        })?;
    let to = app.move_note(path, &folder)?;
    Ok(Moved {
        id: to,
        library: app.reload(&handle),
    })
}

/// Moves the last archived or restored note back where it was.
#[tauri::command]
fn undo(app: State<App>, handle: AppHandle) -> Result<Moved, String> {
    let last = app
        .last_move
        .lock()
        .unwrap()
        .take()
        .ok_or("Nothing to undo")?;
    if !last.to.is_file() {
        return Err(format!("{} is gone", last.to.display()));
    }
    if last.from.exists() {
        return Err(format!("{} exists again", last.from.display()));
    }
    notes::move_file(&last.to, &last.from)?;
    Ok(Moved {
        id: last.from,
        library: app.reload(&handle),
    })
}

#[tauri::command]
fn trash(app: State<App>, handle: AppHandle, id: &str) -> Result<Library, String> {
    notes::trash(&app.resolve(id)?)?;
    Ok(app.reload(&handle))
}

#[tauri::command]
fn session_defaults(app: State<App>, id: &str) -> Result<SessionDefaults, String> {
    let path = app.resolve(id)?;
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let folders: Vec<PathBuf> = app
        .folders
        .lock()
        .unwrap()
        .iter()
        .map(|f| f.path.clone())
        .collect();
    let mut ignore: Vec<PathBuf> = folders
        .iter()
        .filter_map(|f| f.parent().map(Path::to_path_buf))
        .collect();
    ignore.push(config::claude_dir(&app.home));
    let folder = path
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Ok(SessionDefaults {
        workdir: session::guess_workdir(&content, &app.home, &ignore),
        prompt: session::default_prompt(&path, &folder),
    })
}

#[tauri::command]
fn start_session(app: State<App>, workdir: &str, prompt: &str) -> Result<(), String> {
    let workdir = if workdir.starts_with('~') {
        config::expand(workdir, &app.home)
    } else {
        PathBuf::from(workdir)
    };
    session::launch(&workdir, prompt)
}

/// Shows the note in Finder.
#[tauri::command]
fn reveal(app: State<App>, id: &str) -> Result<(), String> {
    let path = app.resolve(id)?;
    open(&["-R".as_ref(), path.as_os_str()])
}

/// Opens the note in the default Markdown editor.
#[tauri::command]
fn edit(app: State<App>, id: &str) -> Result<(), String> {
    let path = app.resolve(id)?;
    open(&[path.as_os_str()])
}

/// The default macOS menu, with Settings… (⌘,) under the app menu.
fn menu(handle: &AppHandle) -> tauri::Result<Menu<tauri::Wry>> {
    let menu = Menu::default(handle)?;
    if let Some(app_menu) = menu.items()?.first().and_then(|item| item.as_submenu()) {
        let settings =
            MenuItem::with_id(handle, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?;
        // After About and its separator.
        app_menu.insert(&settings, 2)?;
        app_menu.insert(&PredefinedMenuItem::separator(handle)?, 3)?;
    }
    Ok(menu)
}

/// Opens a web or mail link in the default browser; any other scheme is refused.
#[tauri::command]
fn open_url(url: &str) -> Result<(), String> {
    let allowed = ["https://", "http://", "mailto:"]
        .iter()
        .any(|s| url.starts_with(s));
    if !allowed {
        return Err(format!("refused to open {url}"));
    }
    open(&[url.as_ref()])
}

fn open(args: &[&OsStr]) -> Result<(), String> {
    let mut cmd = Command::new("/usr/bin/open");
    cmd.args(args);
    let status = cmd.status().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("open exited with {status}"))
    }
}

pub fn run() {
    let home = std::env::home_dir().expect("no home directory");
    tauri::Builder::default()
        .manage(App {
            settings_file: config::settings_file(&home),
            home,
            folders: Mutex::default(),
            notes: Mutex::default(),
            archive: Mutex::default(),
            archive_folders: Mutex::default(),
            archived: Mutex::default(),
            last_move: Mutex::default(),
            watched: Mutex::default(),
            watcher: Mutex::default(),
        })
        .menu(menu)
        .on_menu_event(|handle, event| {
            if event.id() == "settings" {
                let _ = handle.emit("open-settings", ());
            }
        })
        .setup(|app| {
            let state = app.state::<App>();
            let claude_dir = config::claude_dir(&state.home);
            if let Err(e) = config::pin_archive(&state.settings_file, &state.home, &claude_dir) {
                eprintln!("memo: could not save the archive folder: {e}");
            }
            state.reload(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library,
            settings,
            save_settings,
            choose_folder,
            complete_folder,
            open_privacy_settings,
            render,
            search,
            archive,
            restore,
            undo,
            trash,
            session_defaults,
            start_session,
            reveal,
            edit,
            open_url
        ])
        .run(tauri::generate_context!())
        .expect("error while running memo");
}
