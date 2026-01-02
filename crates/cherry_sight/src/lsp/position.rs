// Copyright (c) 2025 Krishna Teja Mekala (Kriaa Systems). All rights reserved.

//! LSP position and range conversion utilities

use crate::util::{FileId, Span};
use lsp_types::{Position, Range};

/// Convert LSP Position (line, character) to byte offset in text
pub fn position_to_offset(text: &str, position: Position) -> Option<usize> {
    let mut line = 0;
    let mut character = 0;
    let mut byte_offset = 0;

    for ch in text.chars() {
        if line == position.line as usize && character == position.character as usize {
            return Some(byte_offset);
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }

        byte_offset += ch.len_utf8();
    }

    // Handle end of file
    if line == position.line as usize && character == position.character as usize {
        Some(byte_offset)
    } else {
        None
    }
}

/// Convert byte offset to LSP Position (line, character)
pub fn offset_to_position(text: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;
    let mut current_offset = 0;

    for ch in text.chars() {
        if current_offset >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }

        current_offset += ch.len_utf8();
    }

    Position {
        line: line as u32,
        character: character as u32,
    }
}

/// Convert our Span to LSP Range
pub fn span_to_range(text: &str, span: Span) -> Range {
    Range {
        start: offset_to_position(text, span.start as usize),
        end: offset_to_position(text, span.end as usize),
    }
}

/// Convert LSP Range to our Span
pub fn range_to_span(text: &str, range: Range, file_id: FileId) -> Option<Span> {
    let start = position_to_offset(text, range.start)? as u32;
    let end = position_to_offset(text, range.end)? as u32;
    Some(Span::new(file_id, start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_to_offset_first_line() {
        let text = "hello world\nfoo bar";
        let pos = Position {
            line: 0,
            character: 6,
        };
        assert_eq!(position_to_offset(text, pos), Some(6));
    }

    #[test]
    fn test_position_to_offset_second_line() {
        let text = "hello world\nfoo bar";
        let pos = Position {
            line: 1,
            character: 4,
        };
        // "hello world\n" = 12 bytes, then "foo " = 4 bytes
        assert_eq!(position_to_offset(text, pos), Some(16));
    }

    #[test]
    fn test_offset_to_position_first_line() {
        let text = "hello world\nfoo bar";
        let pos = offset_to_position(text, 6);
        assert_eq!(
            pos,
            Position {
                line: 0,
                character: 6
            }
        );
    }

    #[test]
    fn test_offset_to_position_second_line() {
        let text = "hello world\nfoo bar";
        let pos = offset_to_position(text, 16);
        assert_eq!(
            pos,
            Position {
                line: 1,
                character: 4
            }
        );
    }

    #[test]
    fn test_round_trip() {
        let text = "hello world\nfoo bar\nbaz";
        let original_pos = Position {
            line: 1,
            character: 4,
        };

        let offset = position_to_offset(text, original_pos).unwrap();
        let converted_pos = offset_to_position(text, offset);

        assert_eq!(original_pos, converted_pos);
    }

    #[test]
    fn test_span_to_range() {
        let text = "hello world\nfoo bar";
        let file_id = FileId::new(1);
        let span = Span::new(file_id, 0, 5); // "hello"

        let range = span_to_range(text, span);
        assert_eq!(
            range,
            Range {
                start: Position {
                    line: 0,
                    character: 0
                },
                end: Position {
                    line: 0,
                    character: 5
                }
            }
        );
    }
}
