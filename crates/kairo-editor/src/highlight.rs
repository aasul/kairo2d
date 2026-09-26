use eframe::egui::{text::LayoutJob, Color32, FontId, TextFormat};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Plain,
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Module,
}

pub fn layout(source: &str, font_size: f32, lua: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    for (range, kind) in tokens(source, lua) {
        let color = match kind {
            Kind::Plain => Color32::from_rgb(213, 219, 229),
            Kind::Keyword => Color32::from_rgb(191, 161, 226),
            Kind::String => Color32::from_rgb(159, 201, 150),
            Kind::Comment => Color32::from_rgb(117, 129, 144),
            Kind::Number => Color32::from_rgb(221, 184, 132),
            Kind::Function => Color32::from_rgb(137, 186, 218),
            Kind::Module => Color32::from_rgb(126, 199, 190),
        };
        job.append(
            &source[range],
            0.0,
            TextFormat {
                font_id: FontId::monospace(font_size),
                color,
                ..Default::default()
            },
        );
    }
    job.wrap.max_width = f32::INFINITY;
    job
}

pub fn tokens(source: &str, lua: bool) -> Vec<(Range<usize>, Kind)> {
    let mut result = Vec::new();
    let mut i = 0;
    let bytes = source.as_bytes();
    while i < bytes.len() {
        let start = i;
        let kind;
        if (lua && source[i..].starts_with("--")) || (!lua && bytes[i] == b'#') {
            i += if lua { 2 } else { 1 };
            i = long_string_end(source, i)
                .unwrap_or_else(|| source[i..].find('\n').map_or(source.len(), |end| i + end));
            kind = Kind::Comment;
        } else if let Some(end) = long_string_end(source, i).filter(|_| lua) {
            i = end;
            kind = Kind::String;
        } else if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            i += 1;
            while i < bytes.len() {
                let byte = bytes[i];
                if byte == b'\\' {
                    i += 1;
                    if i < bytes.len() {
                        i += source[i..].chars().next().map_or(0, char::len_utf8);
                    }
                } else {
                    i += source[i..].chars().next().map_or(0, char::len_utf8);
                    if byte == quote {
                        break;
                    }
                }
            }
            kind = Kind::String;
        } else if bytes[i].is_ascii_digit() {
            i += 1;
            while i < bytes.len()
                && (bytes[i].is_ascii_alphanumeric()
                    || bytes[i] == b'.'
                    || ((bytes[i] == b'+' || bytes[i] == b'-')
                        && matches!(bytes[i - 1], b'e' | b'E' | b'p' | b'P')))
            {
                i += 1;
            }
            kind = Kind::Number;
        } else if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
            i += 1;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            let name = &source[start..i];
            kind = if [
                "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto",
                "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true",
                "until", "while",
            ]
            .contains(&name)
            {
                Kind::Keyword
            } else if [
                "game",
                "graphics",
                "audio",
                "keyboard",
                "mouse",
                "window",
                "timer",
                "filesystem",
                "assets",
                "physics",
                "scene",
                "input",
                "inspector",
                "animation",
                "animator",
                "particles",
                "save",
                "localization",
                "prefab",
                "tilemap",
                "replay",
                "random",
                "ui",
                "debugui",
                "debug",
                "profiler",
                "gamepad",
            ]
            .contains(&name)
            {
                Kind::Module
            } else if source[i..].trim_start().starts_with('(') {
                Kind::Function
            } else {
                Kind::Plain
            };
        } else {
            i += source[i..].chars().next().map_or(1, char::len_utf8);
            kind = Kind::Plain;
        }
        result.push((start..i, kind));
    }
    result
}

fn long_string_end(source: &str, start: usize) -> Option<usize> {
    let rest = source.get(start..)?;
    if !rest.starts_with('[') {
        return None;
    }
    let equals = rest[1..].bytes().take_while(|b| *b == b'=').count();
    let opening = equals + 2;
    if rest.as_bytes().get(opening - 1) != Some(&b'[') {
        return None;
    }
    let closing = format!("]{}]", "=".repeat(equals));
    Some(
        rest[opening..]
            .find(&closing)
            .map_or(source.len(), |i| start + opening + i + closing.len()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lexer_handles_multiline_strings_comments_and_unicode_without_gaps() {
        let source = "--[=[ comment\n ]=]\nlocal x = [==[caf\u{e9}]==]\ngraphics.print(42)";
        let parts = tokens(source, true);
        assert_eq!(
            parts
                .iter()
                .map(|(range, _)| &source[range.clone()])
                .collect::<String>(),
            source
        );
        assert!(parts.iter().any(|(_, kind)| *kind == Kind::Comment));
        assert!(parts.iter().any(|(_, kind)| *kind == Kind::String));
        assert!(parts.iter().any(|(_, kind)| *kind == Kind::Module));
        assert!(parts.iter().any(|(_, kind)| *kind == Kind::Number));
    }

    #[test]
    fn unfinished_strings_remain_editable() {
        for source in ["\"unfinished\\", "--[=[unfinished", "[[text", "\u{1f642}"] {
            let parts = tokens(source, true);
            assert_eq!(parts.last().unwrap().0.end, source.len());
        }
    }
}
