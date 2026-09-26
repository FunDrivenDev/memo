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
use tauri::{AppHandle, Emitter, Manager, State};

use config::Folder;
use notes::Note;

struct App {
    home: PathBuf,
    folders: Mutex<Vec<Folder>>,
    notes: Mutex<Vec<Note>>,
    watcher: Mutex<Option<Debouncer<RecommendedWatcher>>>,
}

impl App {
    /// Rereads the configuration and the folders; watches the folders when they changed.
    fn reload(&self, handle: &AppHandle) -> Library {
        let folders = config::discover(&self.home, &config::claude_dir(&self.home));
        let notes = notes::list(&folders);
        let changed = *self.folders.lock().unwrap() != folders;
        if changed || self.watcher.lock().unwrap().is_none() {
            *self.watcher.lock().unwrap() = watch(handle, &folders);
        }
        *self.folders.lock().unwrap() = folders.clone();
        *self.notes.lock().unwrap() = notes.clone();
        Library {
            claude_dir: config::claude_dir(&self.home),
            folders,
            notes,
        }
    }

    fn resolve(&self, id: &str) -> Result<PathBuf, String> {
        notes::resolve(&self.folders.lock().unwrap(), id)
    }
}

/// Emits `library-changed` when a file of the folders changes.
fn watch(handle: &AppHandle, folders: &[Folder]) -> Option<Debouncer<RecommendedWatcher>> {
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
            .watch(&folder.path, RecursiveMode::NonRecursive);
    }
    Some(debouncer)
}

#[derive(Serialize)]
struct Library {
    claude_dir: PathBuf,
    folders: Vec<Folder>,
    notes: Vec<Note>,
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
fn search(app: State<App>, query: &str) -> Vec<search::Hit> {
    search::search(query, &app.notes.lock().unwrap(), 50)
}

#[tauri::command]
fn archive(app: State<App>, handle: AppHandle, id: &str) -> Result<Library, String> {
    notes::archive(&app.resolve(id)?)?;
    Ok(app.reload(&handle))
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
            home,
            folders: Mutex::default(),
            notes: Mutex::default(),
            watcher: Mutex::default(),
        })
        .setup(|app| {
            let state = app.state::<App>();
            state.reload(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            library,
            render,
            search,
            archive,
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
