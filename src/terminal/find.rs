//! Text in terminal rows: Cmd+F matches and Cmd+click links. Pure: rows in as chars, char ranges out.
use std::ops::Range;

/// Where `query` occurs in `line`, as char ranges, left to right and not overlapping. Smart case: a query with
/// no capitals ignores case.
pub fn find_all(line: &[char], query: &[char]) -> Vec<Range<usize>> {
    let fold = !query.iter().any(|c| c.is_uppercase());
    let same = |a: char, b: char| a == b || (fold && a.to_lowercase().eq(b.to_lowercase()));
    let mut hits = Vec::new();
    let mut i = 0;
    while !query.is_empty() && i + query.len() <= line.len() {
        if line[i..i + query.len()].iter().zip(query).all(|(&a, &b)| same(a, b)) {
            hits.push(i..i + query.len());
            i += query.len();
        } else {
            i += 1;
        }
    }
    hits
}

#[derive(Debug, Clone, PartialEq)]
pub enum Link {
    Url(String),
    File { path: String, line: Option<u32>, col: Option<u32> },
}

/// The URL or path (`src/a.rs:12:5`) under char `at`, with its char range in `line`.
// ponytail: one row; a URL wrapped onto the next row opens cut short.
pub fn link_at(line: &[char], at: usize) -> Option<(Range<usize>, Link)> {
    let stop = |c: char| c.is_whitespace() || "\"'`<>()[]{}|│".contains(c);
    if at >= line.len() || stop(line[at]) {
        return None;
    }
    let mut start = at;
    while start > 0 && !stop(line[start - 1]) {
        start -= 1;
    }
    let mut end = at + 1;
    while end < line.len() && !stop(line[end]) {
        end += 1;
    }
    let token: String = line[start..end].iter().collect();
    // `url=https://…`: the link starts at its scheme.
    if let Some(i) = ["https://", "http://", "file://"].iter().filter_map(|s| token.find(s)).min() {
        let url = token[i..].trim_end_matches(['.', ',', ';', ':', '!', '?']);
        let from = start + token[..i].chars().count();
        let range = from..from + url.chars().count();
        return range.contains(&at).then(|| (range, Link::Url(url.to_string())));
    }
    let trimmed = token.trim_end_matches(['.', ',', ';', ':', '!', '?']);
    let mut path = trimmed;
    let mut nums = Vec::new();
    while let Some((rest, n)) = path.rsplit_once(':').and_then(|(rest, n)| n.parse::<u32>().ok().map(|n| (rest, n))) {
        if nums.len() == 2 {
            break;
        }
        nums.insert(0, n);
        path = rest;
    }
    // A path has a slash, or is a file name with a line (`main.rs:3`); "and/or" passes too, the click checks it.
    let file_name = path.contains('.') && !nums.is_empty();
    if path.is_empty() || !(path.contains('/') || file_name) || path.contains("://") {
        return None;
    }
    let range = start..start + trimmed.chars().count();
    range.contains(&at).then(|| (range, Link::File { path: path.to_string(), line: nums.first().copied(), col: nums.get(1).copied() }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    #[test]
    fn finds_every_match_with_smart_case() {
        assert_eq!(find_all(&chars("Error: error ERROR"), &chars("error")), vec![0..5, 7..12, 13..18]);
        assert_eq!(find_all(&chars("Error: error"), &chars("Error")), vec![0..5]);
        assert_eq!(find_all(&chars("aaaa"), &chars("aa")), vec![0..2, 2..4]);
        assert_eq!(find_all(&chars("ПОИСК поиск"), &chars("поиск")), vec![0..5, 6..11]);
        assert!(find_all(&chars("abc"), &chars("")).is_empty());
    }

    #[test]
    fn finds_urls_and_paths_under_the_pointer() {
        let line = chars("see https://herdr.dev/docs. and (src/main.rs:12:5) then ~/x/y.txt, ok");
        let url = link_at(&line, 10).unwrap();
        assert_eq!(url.1, Link::Url("https://herdr.dev/docs".into()));
        assert_eq!(url.0, 4..26);
        let file = Link::File { path: "src/main.rs".into(), line: Some(12), col: Some(5) };
        assert_eq!(link_at(&line, 35), Some((33..49, file)));
        assert_eq!(link_at(&line, 58).map(|l| l.1), Some(Link::File { path: "~/x/y.txt".into(), line: None, col: None }));
        assert_eq!(link_at(&line, 0), None); // "see"
        assert_eq!(link_at(&line, 3), None); // a space
    }

    #[test]
    fn a_bare_file_name_needs_a_line() {
        assert_eq!(link_at(&chars("lib.rs:7:"), 0).map(|l| l.1), Some(Link::File { path: "lib.rs".into(), line: Some(7), col: None }));
        assert_eq!(link_at(&chars("lib.rs"), 0), None);
        assert_eq!(link_at(&chars("12:30"), 0), None);
    }

    #[test]
    fn a_url_after_a_prefix_starts_at_its_scheme() {
        let line = chars("url=http://a.b/c");
        assert_eq!(link_at(&line, 1), None);
        assert_eq!(link_at(&line, 8), Some((4..16, Link::Url("http://a.b/c".into()))));
    }
}
