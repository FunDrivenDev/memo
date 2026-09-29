# Session folders are remembered, the usual ones first

Where a Claude Code session starts decides what it can see, so `s` asks for the folder as well as the prompt, and memo remembers the folders sessions start in.

- **Which folder is proposed.** The git repository the note mentions most, since a plan usually names the files it is about; else the folder sessions usually start in; else home.
- **Which folders are listed.** The remembered ones, ranked by uses weighted by age (in full for a day, half for a week, a quarter for a month, an eighth after), so a folder used a lot long ago gives way to this week's. Typing filters them by any part of the path, since a project's name is easier to recall than its path; a path is still completed from the disk.
- **Where they are kept.** `workdirs.json` beside memo's settings, 30 at most, the lowest ranked forgotten first but never the one just used. Folders that no longer exist are not listed but are kept, in case they come back.
