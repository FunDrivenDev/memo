# Updates go through Homebrew, never around it

memo is installed from the FunDrivenDev tap, so it tells when a newer version is out and has Homebrew install it, rather than replacing itself.

- **Why not the Tauri updater.** It would swap the bundle under Homebrew, whose records would then name the old version: the cask would need `auto_updates true`, and `brew outdated` would skip memo. It would also need an updater key pair, and a `latest.json`, the `.app.tar.gz` and its signature on every release, for what `brew upgrade` already does.
- **Where the version is read.** In the cask, through GitHub's API: the cask is what `brew upgrade` installs, and it is updated after the release, so a version read from the releases could be offered before Homebrew can install it. raw.githubusercontent.com caches the file for minutes, the API does not. memo looks at launch and every 6 hours, well within the 60 requests an hour GitHub allows without a token.
- **Why in a terminal window.** macOS asks before an app modifies another one in `/Applications`, so brew run by memo itself could be refused, while the terminal you run `brew upgrade` in already has that right. The window also shows what Homebrew does, and what went wrong if it fails.
- **Why memo quits first.** `open` on a running app shows it rather than starting the new version, and brew would move the bundle from under it. The command waits for memo's process to end, upgrades, and reopens it either way, so a failed upgrade still gives memo back.
- **What is Homebrew's memo.** One running from `/Applications`, with memo in Homebrew's Caskroom. Anything else, a copy built with `just app` included, gets the release page instead.

macOS keys the permissions of an ad-hoc signed app to the build, so each update asks again for the terminal and the protected folders, as `brew upgrade` run by hand does. Only a Developer ID signature would end that.
