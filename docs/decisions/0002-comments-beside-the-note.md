# Comments live beside the note, never in it

Comments are kept in `comments.json` in memo's application support folder, not written into the Markdown.

The notes are Claude Code's plans, handoffs and reports: a session may rewrite them at any time, and another reads them as they are. Comments inside them would be lost on a rewrite or read as instructions. memo stays a reader: it never changes a note.

The cost is that a comment must find its text again after the note changes. It keeps the quoted text, a few characters around it and its old lines, and prefers the match nearest those lines; one whose text is gone shows at the top of the note. It follows the note when memo archives, restores or undoes a move, and goes when the note is trashed.
