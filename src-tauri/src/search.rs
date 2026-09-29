//! Fuzzy search over the titles and every line of the notes, and over the comments on
//! them, with nucleo.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::Serialize;

use crate::comments::Comment;
use crate::notes::Note;

/// A matching note, with the character positions to highlight.
#[derive(Debug, Serialize)]
pub struct Hit {
    pub id: String,
    pub score: u32,
    /// Whether the title matches every search word, not only the content.
    pub in_title: bool,
    pub title_indices: Vec<u32>,
    pub snippets: Vec<Snippet>,
}

/// A matching line, cut around the match when long.
#[derive(Debug, Serialize)]
pub struct Snippet {
    /// 1-based line number in the file.
    pub line: usize,
    pub text: String,
    pub indices: Vec<u32>,
}

/// A matching comment.
#[derive(Debug, Serialize)]
pub struct CommentHit {
    pub id: String,
    /// The note it is on.
    pub note: String,
    pub score: u32,
    /// Its matching lines, the line numbers counted in the comment.
    pub snippets: Vec<Snippet>,
}

/// What a search finds: notes, and apart from them, comments.
#[derive(Debug, Serialize)]
pub struct Results {
    pub notes: Vec<Hit>,
    pub comments: Vec<CommentHit>,
}

const SNIPPETS_PER_NOTE: usize = 3;
const SNIPPET_CHARS: usize = 140;

/// The notes matching `query`, as [`search`] ranks them, and the comments on those notes
/// matching it, best first, by the same rules.
pub fn search_all(query: &str, notes: &[Note], comments: &[Comment], limit: usize) -> Results {
    Results {
        notes: search(query, notes, limit),
        comments: search_comments(query, notes, comments, limit),
    }
}

/// The lines of `text` matching `pattern`, best first, then in order; blank ones skipped.
fn matching_lines<'a>(
    pattern: &Pattern,
    text: &'a str,
    matcher: &mut Matcher,
    buf: &mut Vec<char>,
) -> Vec<(u32, usize, &'a str)> {
    let mut lines: Vec<(u32, usize, &str)> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(score) = pattern.score(Utf32Str::new(line, buf), matcher) {
            lines.push((score, n, line));
        }
    }
    lines.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    lines
}

/// A score from the best line, raised a little by how many lines match.
fn score(lines: &[(u32, usize, &str)], floor: u32) -> u32 {
    let best_line = lines.first().map_or(0, |l| l.0);
    let breadth = u32::try_from(lines.len().min(10)).unwrap_or(10);
    floor.max(best_line) + breadth
}

/// The first few matching lines, highlighted.
fn snippets(
    pattern: &Pattern,
    lines: &[(u32, usize, &str)],
    matcher: &mut Matcher,
    buf: &mut Vec<char>,
) -> Vec<Snippet> {
    lines
        .iter()
        .take(SNIPPETS_PER_NOTE)
        .map(|&(_, n, line)| {
            let mut indices = Vec::new();
            pattern.indices(Utf32Str::new(line, buf), matcher, &mut indices);
            indices.sort_unstable();
            indices.dedup();
            snippet(n + 1, line, &indices)
        })
        .collect()
}

/// The comments on `notes` whose text matches `query`, best first.
pub fn search_comments(
    query: &str,
    notes: &[Note],
    comments: &[Comment],
    limit: usize,
) -> Vec<CommentHit> {
    if query.trim().is_empty() {
        return Vec::new();
    }
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();
    let mut hits: Vec<CommentHit> = comments
        .iter()
        .filter(|c| notes.iter().any(|n| n.id == c.note))
        .filter_map(|comment| {
            let lines = matching_lines(&pattern, &comment.body, &mut matcher, &mut buf);
            (!lines.is_empty()).then(|| CommentHit {
                id: comment.id.clone(),
                note: comment.note.clone(),
                score: score(&lines, 0),
                snippets: snippets(&pattern, &lines, &mut matcher, &mut buf),
            })
        })
        .collect();
    hits.sort_by_key(|h| std::cmp::Reverse(h.score));
    hits.truncate(limit);
    hits
}

/// The notes matching `query`: those whose title matches first, then those matching in the
/// content only, each group best first.
pub fn search(query: &str, notes: &[Note], limit: usize) -> Vec<Hit> {
    let pattern = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart);
    if query.trim().is_empty() {
        return Vec::new();
    }
    let mut matcher = Matcher::new(Config::DEFAULT);
    let mut buf = Vec::new();
    let mut hits = Vec::new();

    for note in notes {
        let mut title_indices = Vec::new();
        let title_score = pattern.indices(
            Utf32Str::new(&note.title, &mut buf),
            &mut matcher,
            &mut title_indices,
        );
        title_indices.sort_unstable();
        title_indices.dedup();

        let lines = matching_lines(&pattern, &note.content, &mut matcher, &mut buf);
        if title_score.is_none() && lines.is_empty() {
            continue;
        }
        hits.push(Hit {
            id: note.id.clone(),
            score: score(&lines, title_score.unwrap_or(0)),
            in_title: title_score.is_some(),
            title_indices,
            snippets: snippets(&pattern, &lines, &mut matcher, &mut buf),
        });
    }
    hits.sort_by_key(|h| std::cmp::Reverse((h.in_title, h.score)));
    hits.truncate(limit);
    hits
}

/// The line trimmed, and cut to a window starting a little before the first match.
fn snippet(line: usize, text: &str, indices: &[u32]) -> Snippet {
    let chars: Vec<char> = text.chars().collect();
    let lead = chars.iter().take_while(|c| c.is_whitespace()).count();
    let first = indices.first().map_or(lead, |&i| i as usize);
    let start = if chars.len() - lead <= SNIPPET_CHARS {
        lead
    } else {
        first.saturating_sub(30).max(lead)
    };
    let end = (start + SNIPPET_CHARS).min(chars.len());

    let mut out = String::new();
    let mut offset = 0u32;
    if start > lead {
        out.push('…');
        offset = 1;
    }
    out.extend(&chars[start..end]);
    if end < chars.len() {
        out.push('…');
    }
    let indices = indices
        .iter()
        .filter(|&&i| (i as usize) >= start && (i as usize) < end)
        .map(|&i| i - start as u32 + offset)
        .collect();
    Snippet {
        line,
        text: out,
        indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(id: &str, title: &str, content: &str) -> Note {
        Note {
            id: id.into(),
            folder: "plans".into(),
            file_name: format!("{id}.md"),
            title: title.into(),
            excerpt: String::new(),
            modified: 0,
            content: content.into(),
        }
    }

    #[test]
    fn finds_content_and_ranks_titles_higher() {
        let notes = [
            note(
                "a",
                "Terraform layer",
                "# Terraform layer\nprovision the vps",
            ),
            note(
                "b",
                "Ansible audit",
                "# Ansible audit\nwe also run terraform plan",
            ),
            note("c", "Unrelated", "# Unrelated\nnothing here"),
        ];
        let hits = search("terraform", &notes, 10);
        let ids: Vec<&str> = hits.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(hits[1].snippets[0].line, 2);
        assert_eq!(hits[1].snippets[0].text, "we also run terraform plan");
        assert!(hits[0].in_title && !hits[1].in_title);
    }

    #[test]
    fn a_weak_title_match_outranks_a_strong_content_match() {
        let notes = [
            note(
                "body",
                "Ansible audit",
                "# Ansible audit\nterraform terraform terraform",
            ),
            note(
                "title",
                "The platform refactor",
                "# The platform refactor\nnothing",
            ),
        ];
        let ids: Vec<String> = search("tfrm", &notes, 10)
            .into_iter()
            .map(|h| h.id)
            .collect();
        assert_eq!(ids, ["title", "body"]);
    }

    #[test]
    fn is_fuzzy_and_needs_every_word() {
        let notes = [
            note("a", "T", "bootstrap the first run"),
            note("b", "T", "bootstrap only"),
        ];
        assert_eq!(search("bstp fst", &notes, 10).len(), 1);
        assert!(search("   ", &notes, 10).is_empty());
    }

    fn comment(id: &str, note: &str, body: &str) -> Comment {
        Comment {
            id: id.into(),
            note: note.into(),
            body: body.into(),
            anchor: crate::comments::Anchor {
                kind: "block".into(),
                start: 1,
                end: 1,
                quote: String::new(),
                prefix: String::new(),
                suffix: String::new(),
            },
            created: 0,
            updated: 0,
        }
    }

    #[test]
    fn finds_comments_on_the_notes_searched_exact_matches_first() {
        let notes = [note("a", "Plan", "# Plan\nnothing to see")];
        let comments = [
            comment("fuzzy", "a", "rethink, or consider it"),
            comment("exact", "a", "first line\nreconsider the cache"),
            comment("elsewhere", "archived", "reconsider the cache"),
            comment("none", "a", "unrelated"),
        ];
        let results = search_all("reconsider", &notes, &comments, 10);
        assert!(results.notes.is_empty());
        let ids: Vec<&str> = results.comments.iter().map(|h| h.id.as_str()).collect();
        assert_eq!(ids, ["exact", "fuzzy"]);
        assert_eq!(results.comments[0].snippets[0].line, 2);
        assert_eq!(results.comments[0].snippets[0].text, "reconsider the cache");
        // fzf's syntax, as for the notes.
        let exact = search_comments("'reconsider", &notes, &comments, 10);
        assert_eq!(exact.len(), 1);
    }

    #[test]
    fn snippet_windows_long_lines() {
        let line = format!("{} needle {}", "x".repeat(300), "y".repeat(300));
        let at = 301u32;
        let s = snippet(1, &line, &[at, at + 1]);
        assert!(s.text.starts_with('…') && s.text.ends_with('…'));
        let chars: Vec<char> = s.text.chars().collect();
        assert_eq!(chars[s.indices[0] as usize], 'n');
        assert_eq!(chars[s.indices[1] as usize], 'e');
    }
}
