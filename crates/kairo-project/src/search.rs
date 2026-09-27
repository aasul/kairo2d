use crate::ProjectFiles;
use anyhow::{ensure, Result};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub path: PathBuf,
    pub line: usize,
    pub preview: String,
}
#[derive(Default)]
pub struct SearchReport {
    pub hits: Vec<SearchHit>,
    pub scanned: usize,
    pub limited: bool,
}

/// A bounded literal search. The caller can run this on a worker thread.
pub fn search(files: &ProjectFiles, query: &str, case_sensitive: bool) -> Result<SearchReport> {
    ensure!(
        !query.is_empty() && query.len() <= 256,
        "search text must contain 1..256 bytes"
    );
    let needle = if case_sensitive {
        query.to_owned()
    } else {
        query.to_lowercase()
    };
    let mut report = SearchReport::default();
    let mut bytes = 0u64;
    for node in files.list()? {
        let extension = node
            .relative
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or_default();
        if node.directory
            || !["lua", "toml", "json", "txt", "md", "wgsl"].contains(&extension)
            || node.bytes > 1024 * 1024
        {
            continue;
        }
        bytes = bytes.saturating_add(node.bytes);
        if bytes > 64 * 1024 * 1024 {
            report.limited = true;
            break;
        }
        let source = files.read(&node.relative)?;
        let Ok(source) = std::str::from_utf8(&source) else {
            continue;
        };
        report.scanned += 1;
        for (index, line) in source.lines().enumerate() {
            let matches = if case_sensitive {
                line.contains(&needle)
            } else {
                line.to_lowercase().contains(&needle)
            };
            if matches {
                report.hits.push(SearchHit {
                    path: node.relative.clone(),
                    line: index + 1,
                    preview: line.chars().take(240).collect(),
                });
                if report.hits.len() >= 1000 {
                    report.limited = true;
                    return Ok(report);
                }
            }
        }
    }
    Ok(report)
}

/// Unicode-safe subsequence matching with consecutive/basename bonuses.
pub fn fuzzy_score(query: &str, candidate: &str) -> Option<i64> {
    let needle: Vec<char> = query
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let haystack: Vec<char> = candidate.to_lowercase().chars().collect();
    if needle.is_empty() {
        return Some(0);
    }
    let mut cursor = 0;
    let mut previous = None;
    let mut score = 0;
    for character in needle {
        let offset = haystack[cursor..].iter().position(|c| *c == character)?;
        let index = cursor + offset;
        score += 10 - offset.min(20) as i64;
        if previous == index.checked_sub(1) {
            score += 12;
        }
        if index == 0 || matches!(haystack[index - 1], '/' | '\\' | '_' | '-' | ' ' | '.') {
            score += 8;
        }
        previous = Some(index);
        cursor = index + 1;
    }
    Some(score - haystack.len().min(200) as i64 / 4)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fuzzy_paths_and_unicode_do_not_require_contiguous_bytes() {
        assert!(fuzzy_score("pctrl", "scripts/player_controller.lua").is_some());
        assert!(fuzzy_score("zz", "main.lua").is_none());
        assert!(fuzzy_score("caf", "assets/caf\u{00e9}.lua").is_some());
        assert!(fuzzy_score("main", "main.lua") > fuzzy_score("main", "many/assets/in/nested.lua"));
    }
    #[test]
    fn project_search_ignores_outputs_binary_and_returns_line_numbers() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files
            .create_file("main.lua", b"local speed = 10\n-- NEEDLE\n")
            .unwrap();
        for folder in ["target", "dist", ".git"] {
            std::fs::create_dir(root.path().join(folder)).unwrap();
            std::fs::write(root.path().join(folder).join("ignored.lua"), b"needle").unwrap();
        }
        files.create_file("binary.png", b"needle").unwrap();
        let result = search(&files, "needle", false).unwrap();
        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.hits[0].line, 2);
        assert_eq!(result.hits[0].path, PathBuf::from("main.lua"));
        assert!(search(&files, "", true).is_err());
    }
}
