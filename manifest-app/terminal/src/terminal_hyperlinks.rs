//! URL detection for terminal hyperlinks.
//!
//! Detects URLs in terminal grid content for Cmd+click navigation.

use alacritty_terminal::{
    Term,
    event::EventListener,
    grid::Dimensions,
    index::{Boundary, Column, Direction as AlacDirection, Line, Point as AlacPoint},
    term::{
        cell::Flags,
        search::{Match, RegexIter, RegexSearch},
    },
};
use std::ops::Index;

/// Regex pattern for detecting URLs in terminal output.
/// Matches common URL schemes like http, https, file, etc.
const URL_REGEX: &str = r#"(ipfs:|ipns:|magnet:|mailto:|gemini://|gopher://|https://|http://|news:|file://|git://|ssh:|ftp://)[^\u{0000}-\u{001F}\u{007F}-\u{009F}<>"\s{-}\^⟨⟩`']+"#;

/// Holds the compiled regex for URL searching.
pub struct UrlSearch {
    url_regex: RegexSearch,
}

impl Default for UrlSearch {
    fn default() -> Self {
        Self {
            url_regex: RegexSearch::new(URL_REGEX).expect("URL regex should be valid"),
        }
    }
}

impl UrlSearch {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Find a URL at the given grid point, if any.
///
/// Returns the URL string and the match range if found.
pub fn find_url_at_point<T: EventListener>(
    term: &Term<T>,
    point: AlacPoint,
    url_search: &mut UrlSearch,
) -> Option<(String, Match)> {
    let grid = term.grid();

    // First check if the cell has an explicit hyperlink (OSC 8)
    if let Some(link) = grid.index(point).hyperlink() {
        // Find the bounds of the hyperlink
        let mut min_index = point;
        loop {
            let new_min_index = min_index.sub(term, Boundary::Cursor, 1);
            if new_min_index == min_index
                || grid.index(new_min_index).hyperlink() != Some(link.clone())
            {
                break;
            }
            min_index = new_min_index;
        }

        let mut max_index = point;
        loop {
            let new_max_index = max_index.add(term, Boundary::Cursor, 1);
            if new_max_index == max_index
                || grid.index(new_max_index).hyperlink() != Some(link.clone())
            {
                break;
            }
            max_index = new_max_index;
        }

        let url = link.uri().to_owned();
        let url_match = min_index..=max_index;
        return Some((url, url_match));
    }

    // Otherwise, search for URLs using regex
    let (line_start, line_end) = (term.line_search_left(point), term.line_search_right(point));

    RegexIter::new(
        line_start,
        line_end,
        AlacDirection::Right,
        term,
        &mut url_search.url_regex,
    )
    .find(|rm| rm.contains(&point))
    .map(|url_match| {
        let url = term.bounds_to_string(*url_match.start(), *url_match.end());
        let (url, url_match) = sanitize_url_punctuation(url, url_match, term);
        (url, url_match)
    })
}

/// Remove trailing punctuation that's likely not part of the URL.
fn sanitize_url_punctuation<T: EventListener>(
    url: String,
    url_match: Match,
    term: &Term<T>,
) -> (String, Match) {
    let mut sanitized_url = url;
    let mut chars_trimmed = 0;

    // Count parentheses in the URL
    let (open_parens, mut close_parens) =
        sanitized_url
            .chars()
            .fold((0, 0), |(opens, closes), c| match c {
                '(' => (opens + 1, closes),
                ')' => (opens, closes + 1),
                _ => (opens, closes),
            });

    // Remove trailing characters that shouldn't be at the end of URLs
    while let Some(last_char) = sanitized_url.chars().last() {
        let should_remove = match last_char {
            // These may be part of a URL but not at the end
            '.' | ',' | ':' | ';' => true,
            '(' => true,
            ')' if close_parens > open_parens => {
                close_parens -= 1;
                true
            }
            _ => false,
        };

        if should_remove {
            sanitized_url.pop();
            chars_trimmed += 1;
        } else {
            break;
        }
    }

    if chars_trimmed > 0 {
        let new_end = url_match.end().sub(term, Boundary::Grid, chars_trimmed);
        let sanitized_match = Match::new(*url_match.start(), new_end);
        (sanitized_url, sanitized_match)
    } else {
        (sanitized_url, url_match)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_regex_compiles() {
        let search = UrlSearch::new();
        // Just verify it compiles without panic
        assert!(true);
    }
}
