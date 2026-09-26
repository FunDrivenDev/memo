//! Fuzzy search over the titles and every line of the notes, with nucleo.

use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use serde::Serialize;

use crate::notes::Note;

/// A matching note, with the character positions to highlight.
#[derive(Debug, Serialize)]
pub struct Hit {
    pub id: String,
    pub score: u32,
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

const SNIPPETS_PER_NOTE: usize = 3;
const SNIPPET_CHARS: usize = 140;

/// The notes matching `query`, best first. A title match counts double.
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

        let mut lines: Vec<(u32, usize, &str)> = Vec::new();
        for (n, line) in note.content.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            if let Some(score) = pattern.score(Utf32Str::new(line, &mut buf), &mut matcher) {
                lines.push((score, n, line));
            }
        }
        if title_score.is_none() && lines.is_empty() {
            continue;
        }
        lines.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));

        let best_line = lines.first().map_or(0, |l| l.0);
        let breadth = u32::try_from(lines.len().min(10)).unwrap_or(10);
        let score = title_score.map_or(0, |s| s * 2).max(best_line) + breadth;

        let snippets = lines
            .iter()
            .take(SNIPPETS_PER_NOTE)
            .map(|&(_, n, line)| {
                let mut indices = Vec::new();
                pattern.indices(Utf32Str::new(line, &mut buf), &mut matcher, &mut indices);
                indices.sort_unstable();
                indices.dedup();
                snippet(n + 1, line, &indices)
            })
            .collect();

        hits.push(Hit {
            id: note.id.clone(),
            score,
            title_indices,
            snippets,
        });
    }
    hits.sort_by_key(|h| std::cmp::Reverse(h.score));
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
