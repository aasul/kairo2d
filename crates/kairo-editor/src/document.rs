use anyhow::{ensure, Context, Result};
use kairo_project::ProjectFiles;
use std::collections::VecDeque;
use std::ops::Range;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
const HISTORY_LIMIT: usize = 32;

#[derive(Clone)]
struct Snapshot {
    text: String,
    selection: Range<usize>,
}

pub struct Document {
    pub path: PathBuf,
    pub text: String,
    pub selection: Range<usize>,
    pub pending_selection: Option<Range<usize>>,
    saved: String,
    disk: Vec<u8>,
    crlf: bool,
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: Option<Instant>,
}

impl Document {
    pub fn load(files: &ProjectFiles, path: PathBuf) -> Result<Self> {
        let disk = files.read(&path)?;
        ensure!(
            disk.len() <= MAX_TEXT_BYTES,
            "code editor files must be at most 2 MiB"
        );
        let source = std::str::from_utf8(&disk).context("this file is not UTF-8 text")?;
        let crlf = source.contains("\r\n");
        let text = source.replace("\r\n", "\n");
        Ok(Self {
            path,
            saved: text.clone(),
            text,
            disk,
            crlf,
            selection: 0..0,
            pending_selection: None,
            undo: VecDeque::new(),
            redo: Vec::new(),
            last_edit: None,
        })
    }

    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }

    pub fn save(&mut self, files: &ProjectFiles) -> Result<()> {
        ensure!(
            self.text.len() <= MAX_TEXT_BYTES,
            "text exceeds the 2 MiB editor limit"
        );
        ensure!(
            files.read(&self.path)? == self.disk,
            "{} changed on disk; reload it or save a copy before overwriting",
            self.path.display()
        );
        let bytes = if self.crlf {
            self.text.replace('\n', "\r\n").into_bytes()
        } else {
            self.text.as_bytes().to_vec()
        };
        files.write(&self.path, &bytes)?;
        self.disk = bytes;
        self.saved = self.text.clone();
        Ok(())
    }

    pub fn reload(&mut self, files: &ProjectFiles) -> Result<()> {
        ensure!(
            !self.dirty(),
            "save or discard changes before reloading this tab"
        );
        *self = Self::load(files, self.path.clone())?;
        Ok(())
    }

    pub fn record_edit(&mut self, previous: String, selection: Range<usize>, coalesce: bool) {
        if previous == self.text {
            return;
        }
        if !coalesce
            || self
                .last_edit
                .is_none_or(|time| time.elapsed() > Duration::from_millis(500))
        {
            self.undo.push_back(Snapshot {
                text: previous,
                selection,
            });
            while self.undo.len() > HISTORY_LIMIT {
                self.undo.pop_front();
            }
        }
        self.redo.clear();
        self.last_edit = if coalesce { Some(Instant::now()) } else { None };
    }

    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop_back() {
            self.redo.push(Snapshot {
                text: self.text.clone(),
                selection: self.selection.clone(),
            });
            self.restore(previous);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push_back(Snapshot {
                text: self.text.clone(),
                selection: self.selection.clone(),
            });
            self.restore(next);
        }
    }

    fn restore(&mut self, snapshot: Snapshot) {
        self.text = snapshot.text;
        self.selection = snapshot.selection;
        self.pending_selection = Some(self.selection.clone());
        self.last_edit = None;
    }

    pub fn replace_selection(&mut self, replacement: &str) {
        let before = self.text.clone();
        let selection = self.selection.clone();
        let start = byte_at_char(&self.text, selection.start);
        let end = byte_at_char(&self.text, selection.end);
        self.text.replace_range(start..end, replacement);
        let cursor = selection.start + replacement.chars().count();
        self.selection = cursor..cursor;
        self.pending_selection = Some(self.selection.clone());
        self.record_edit(before, selection, false);
    }

    pub fn insert_newline(&mut self) {
        let offset = byte_at_char(&self.text, self.selection.start);
        let line = self.text[..offset].rsplit('\n').next().unwrap_or("");
        let indent: String = line
            .chars()
            .take_while(|c| *c == ' ' || *c == '\t')
            .collect();
        let code = line.trim_end();
        let opens = code.ends_with("then")
            || code.ends_with("do")
            || code.ends_with("repeat")
            || code.ends_with('{')
            || (code.contains("function") && code.ends_with(')'));
        let extra = if opens { "    " } else { "" };
        self.replace_selection(&format!("\n{indent}{extra}"));
    }

    pub fn indent(&mut self, unindent: bool) {
        let before = self.text.clone();
        let selection = self.selection.clone();
        let start_byte = byte_at_char(&self.text, selection.start);
        let end_byte = byte_at_char(&self.text, selection.end);
        let start = self.text[..start_byte].rfind('\n').map_or(0, |i| i + 1);
        let end = if end_byte > start && self.text[..end_byte].ends_with('\n') {
            end_byte - 1
        } else {
            self.text[end_byte..]
                .find('\n')
                .map_or(self.text.len(), |i| end_byte + i)
        };
        if !unindent && selection.is_empty() {
            self.replace_selection("    ");
            return;
        }
        let block = self.text[start..end]
            .split('\n')
            .map(|line| {
                if unindent {
                    let remove = if line.starts_with('\t') {
                        1
                    } else {
                        line.bytes().take(4).take_while(|b| *b == b' ').count()
                    };
                    line[remove..].to_owned()
                } else {
                    format!("    {line}")
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let start_char = self.text[..start].chars().count();
        self.text.replace_range(start..end, &block);
        self.selection = start_char..start_char + block.chars().count();
        self.pending_selection = Some(self.selection.clone());
        self.record_edit(before, selection, false);
    }

    fn selected_lines(&self) -> Range<usize> {
        let a = byte_at_char(&self.text, self.selection.start);
        let b = byte_at_char(&self.text, self.selection.end);
        let start = self.text[..a].rfind('\n').map_or(0, |i| i + 1);
        let end = if b > start && self.text[..b].ends_with('\n') {
            b - 1
        } else {
            self.text[b..].find('\n').map_or(self.text.len(), |i| b + i)
        };
        start..end
    }

    pub fn toggle_comment(&mut self) {
        let range = self.selected_lines();
        let block = &self.text[range.clone()];
        let uncomment = block
            .lines()
            .filter(|line| !line.trim().is_empty())
            .all(|line| line.trim_start().starts_with("--"));
        let replacement = block
            .split('\n')
            .map(|line| {
                let indent = line.len() - line.trim_start_matches([' ', '\t']).len();
                if uncomment {
                    if let Some(code) = line[indent..].strip_prefix("--") {
                        format!(
                            "{}{}",
                            &line[..indent],
                            code.strip_prefix(' ').unwrap_or(code)
                        )
                    } else {
                        line.to_owned()
                    }
                } else {
                    format!("{}-- {}", &line[..indent], &line[indent..])
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        self.selection =
            self.text[..range.start].chars().count()..self.text[..range.end].chars().count();
        self.replace_selection(&replacement);
    }

    pub fn duplicate_lines(&mut self) {
        let range = self.selected_lines();
        let duplicate = format!("\n{}", &self.text[range.clone()]);
        let end = self.text[..range.end].chars().count();
        self.selection = end..end;
        self.replace_selection(&duplicate);
    }

    pub fn find_next(&mut self, query: &str) -> bool {
        if query.is_empty() {
            return false;
        }
        let after = byte_at_char(&self.text, self.selection.end);
        let found = self.text[after..]
            .find(query)
            .map(|i| i + after)
            .or_else(|| self.text[..after].find(query));
        if let Some(start) = found {
            let start = self.text[..start].chars().count();
            self.selection = start..start + query.chars().count();
            self.pending_selection = Some(self.selection.clone());
            true
        } else {
            false
        }
    }

    pub fn replace_all(&mut self, query: &str, replacement: &str) -> usize {
        if query.is_empty() {
            return 0;
        }
        let count = self.text.matches(query).count();
        let before = self.text.clone();
        let selection = self.selection.clone();
        self.text = self.text.replace(query, replacement);
        self.selection = 0..0;
        self.pending_selection = Some(0..0);
        self.record_edit(before, selection, false);
        count
    }

    pub fn goto_line(&mut self, line: usize) {
        let index = self
            .text
            .split_inclusive('\n')
            .take(line.saturating_sub(1))
            .map(|s| s.chars().count())
            .sum();
        self.selection = index..index;
        self.pending_selection = Some(self.selection.clone());
    }

    pub fn line_column(&self) -> (usize, usize) {
        let prefix = &self.text[..byte_at_char(&self.text, self.selection.start)];
        (
            prefix.bytes().filter(|b| *b == b'\n').count() + 1,
            prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1,
        )
    }
}

pub fn byte_at_char(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(index, _)| index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(text: &str) -> (tempfile::TempDir, ProjectFiles, Document) {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files.create_file("main.lua", text.as_bytes()).unwrap();
        let doc = Document::load(&files, "main.lua".into()).unwrap();
        (root, files, doc)
    }

    #[test]
    fn edits_undo_redo_and_save_real_text() {
        let (_root, files, mut doc) = fixture("return 1\n");
        doc.selection = 7..8;
        doc.replace_selection("42");
        assert!(doc.dirty());
        doc.undo();
        assert_eq!(doc.text, "return 1\n");
        doc.redo();
        doc.save(&files).unwrap();
        assert_eq!(files.read("main.lua").unwrap(), b"return 42\n");
        assert!(!doc.dirty());
    }

    #[test]
    fn unicode_find_uses_character_indices() {
        let (_root, _files, mut doc) = fixture("-- caf\u{e9}\nprint('caf\u{e9}')\n");
        assert!(doc.find_next("caf\u{e9}"));
        doc.replace_selection("tea");
        assert!(doc.text.starts_with("-- tea\n"));
        assert_eq!(doc.replace_all("caf\u{e9}", "water"), 1);
    }

    #[test]
    fn saves_preserve_crlf_and_detect_external_edits() {
        let (_root, files, mut doc) = fixture("return 1\r\n");
        doc.selection = 7..8;
        doc.replace_selection("2");
        doc.save(&files).unwrap();
        assert_eq!(files.read("main.lua").unwrap(), b"return 2\r\n");
        files.write("main.lua", b"external").unwrap();
        assert!(doc.save(&files).is_err());
    }

    #[test]
    fn newline_indent_and_multiline_outdent_are_reversible() {
        let (_root, _files, mut doc) = fixture("if true then");
        let end = doc.text.chars().count();
        doc.selection = end..end;
        doc.insert_newline();
        assert_eq!(doc.text, "if true then\n    ");
        doc.undo();
        assert_eq!(doc.text, "if true then");
        doc.selection = 0..doc.text.chars().count();
        doc.indent(false);
        assert_eq!(doc.text, "    if true then");
        doc.indent(true);
        assert_eq!(doc.text, "if true then");
    }
}

#[cfg(test)]
mod line_edit_tests {
    use super::*;
    #[test]
    fn comments_and_duplicates_are_utf8_safe_and_undoable() {
        let root = tempfile::tempdir().unwrap();
        let files = ProjectFiles::open(root.path()).unwrap();
        files
            .create_file("main.lua", "    print('caf\u{e9}')\n".as_bytes())
            .unwrap();
        let mut doc = Document::load(&files, "main.lua".into()).unwrap();
        doc.toggle_comment();
        assert!(doc.text.starts_with("    -- print"));
        doc.toggle_comment();
        assert!(doc.text.starts_with("    print"));
        doc.duplicate_lines();
        assert_eq!(doc.text.lines().count(), 2);
        doc.undo();
        assert_eq!(doc.text.lines().count(), 1);
    }
}
