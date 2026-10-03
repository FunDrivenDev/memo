//! Tells when a newer memo is out, and has Homebrew install it.
//!
//! memo ships only through the FunDrivenDev tap, so the version its cask names is the one
//! `brew upgrade` installs: memo reads that rather than its releases, which come out
//! before the cask follows.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::session::shell_quote;

/// The cask, read through the API rather than raw.githubusercontent.com, which caches it
/// for minutes.
const CASK_URL: &str =
    "https://api.github.com/repos/FunDrivenDev/homebrew-tap/contents/Casks/memo.rb";
const CASK: &str = "fundrivendev/tap/memo";
const RELEASE_URL: &str = "https://github.com/FunDrivenDev/memo/releases/tag/v";
/// Where Homebrew lives on Apple silicon, then on Intel.
const PREFIXES: [&str; 2] = ["/opt/homebrew", "/usr/local"];

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct Update {
    pub version: String,
    /// Its release notes.
    pub url: String,
    /// Whether Homebrew installed this memo, and can upgrade it.
    pub homebrew: bool,
}

/// A newer version than `current` in the cask, if any.
pub fn newer(current: &str, cask: &str, homebrew: bool) -> Option<Update> {
    let latest = cask_version(cask)?;
    (parse(latest)? > parse(current)?).then(|| Update {
        version: latest.to_string(),
        url: format!("{RELEASE_URL}{latest}"),
        homebrew,
    })
}

/// The version a cask names, on its `version "x.y.z"` line.
fn cask_version(cask: &str) -> Option<&str> {
    cask.lines()
        .find_map(|line| line.trim().strip_prefix("version \"")?.strip_suffix('"'))
}

fn parse(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|p| p.parse().ok());
    let parsed = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(parsed)
}

/// The cask as the tap holds it now, fetched with curl.
pub fn fetch_cask() -> Result<String, String> {
    let output = Command::new("/usr/bin/curl")
        .args(["--fail", "--silent", "--show-error", "--max-time", "15"])
        .args(["--header", "Accept: application/vnd.github.raw+json"])
        .arg(CASK_URL)
        .output()
        .map_err(|e| format!("curl: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// The app bundle memo runs from.
pub fn bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    exe.ancestors()
        .find(|p| p.extension().is_some_and(|e| e == "app"))
        .map(Path::to_path_buf)
}

/// The `brew` that installed the memo of `bundle`: one whose Caskroom holds memo, while
/// memo runs from `/Applications`, where the cask puts it.
pub fn homebrew(bundle: &Path) -> Option<PathBuf> {
    if bundle.parent() != Some(Path::new("/Applications")) {
        return None;
    }
    brew_in(&PREFIXES.map(Path::new))
}

fn brew_in(prefixes: &[&Path]) -> Option<PathBuf> {
    prefixes
        .iter()
        .find(|p| p.join("Caskroom/memo").is_dir() && p.join("bin/brew").is_file())
        .map(|p| p.join("bin/brew"))
}

/// What the terminal runs: it waits for memo (process `pid`) to quit, so that `open`
/// starts the new one rather than showing the old, upgrades it, and reopens it, upgraded
/// or not.
pub fn command(brew: &Path, bundle: &Path, pid: u32) -> String {
    let brew = shell_quote(&brew.to_string_lossy());
    let bundle = shell_quote(&bundle.to_string_lossy());
    format!(
        "while kill -0 {pid} 2>/dev/null; do sleep 0.2; done; \
         {brew} update --quiet; {brew} upgrade --cask {CASK}; open {bundle}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const CASK_FILE: &str = "cask \"memo\" do\n  version \"0.2.10\"\n  sha256 \"abc\"\nend\n";

    #[test]
    fn finds_a_newer_version_in_the_cask() {
        assert_eq!(
            newer("0.2.9", CASK_FILE, true),
            Some(Update {
                version: "0.2.10".into(),
                url: "https://github.com/FunDrivenDev/memo/releases/tag/v0.2.10".into(),
                homebrew: true,
            })
        );
        assert_eq!(newer("0.2.10", CASK_FILE, true), None);
        assert_eq!(newer("1.0.0", CASK_FILE, false), None);
    }

    #[test]
    fn ignores_what_it_cannot_read() {
        assert_eq!(newer("0.1.0", "<html>rate limited</html>", true), None);
        assert_eq!(newer("0.1.0", "  version \"0.2\"\n", true), None);
        assert_eq!(newer("0.1.0-dev", CASK_FILE, true), None);
    }

    #[test]
    fn finds_the_brew_that_installed_memo() {
        let root = tempfile::tempdir().unwrap();
        let (other, owner) = (root.path().join("other"), root.path().join("owner"));
        std::fs::create_dir_all(other.join("bin")).unwrap();
        std::fs::write(other.join("bin/brew"), "").unwrap();
        assert_eq!(brew_in(&[&other, &owner]), None);
        std::fs::create_dir_all(owner.join("Caskroom/memo")).unwrap();
        std::fs::create_dir_all(owner.join("bin")).unwrap();
        std::fs::write(owner.join("bin/brew"), "").unwrap();
        assert_eq!(brew_in(&[&other, &owner]), Some(owner.join("bin/brew")));
    }

    #[test]
    fn upgrades_only_the_memo_of_applications() {
        assert_eq!(homebrew(Path::new("/Users/me/Applications/memo.app")), None);
    }

    #[test]
    fn upgrades_once_memo_has_quit_then_reopens_it() {
        let command = command(
            Path::new("/opt/homebrew/bin/brew"),
            Path::new("/Applications/memo.app"),
            42,
        );
        assert_eq!(
            command,
            "while kill -0 42 2>/dev/null; do sleep 0.2; done; \
             '/opt/homebrew/bin/brew' update --quiet; \
             '/opt/homebrew/bin/brew' upgrade --cask fundrivendev/tap/memo; \
             open '/Applications/memo.app'"
        );
    }
}
