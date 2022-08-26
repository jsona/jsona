//! Utilities for mapping between offset:length bytes and col:row character positions.

use jsona::rowan::{TextRange, TextSize};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Position {
    /// Cursor position in a document
    pub index: u64,
    /// Line position in a document (could be zero-based or one-based based on the usage).
    pub line: u64,
    /// Character offset on a line in a document (could be zero-based or one-based based on the usage).
    pub character: u64,
}

impl Position {
    #[must_use]
    pub fn new(index: u64, line: u64, character: u64) -> Self {
        Position {
            index,
            line,
            character,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone, Default, Serialize, Deserialize)]
pub struct Range {
    /// The range's start position.
    pub start: Position,
    /// The range's end position.
    pub end: Position,
}

/// Inclusive offset range in characters instead of bytes.
#[derive(Debug, Clone, Copy)]
pub struct CharacterRange(u64, u64);

/// A mapper that translates offset:length bytes to
/// 1-based line:row characters.
#[derive(Debug, Clone)]
pub struct Mapper {
    /// Mapping offsets to positions.
    offset_to_position: BTreeMap<TextSize, Position>,

    /// Mapping positions to offsets.
    position_to_offset: BTreeMap<Position, TextSize>,

    /// Line count.
    lines: usize,

    /// Ending position.
    end: Position,
}

impl Mapper {
    /// Creates a new Mapper that remembers where
    /// each line starts and ends.
    ///
    /// Uses UTF-16 character sizes for positions.
    #[must_use]
    pub fn new_utf16(source: &str, one_based: bool) -> Self {
        Self::new_impl(source, true, if one_based { 1 } else { 0 })
    }

    /// Uses UTF-8 character sizes for positions.
    #[must_use]
    pub fn new_utf8(source: &str, one_based: bool) -> Self {
        Self::new_impl(source, false, if one_based { 1 } else { 0 })
    }

    #[must_use]
    pub fn offset(&self, position: Position) -> Option<TextSize> {
        self.position_to_offset.get(&position).copied()
    }

    #[must_use]
    pub fn text_range(&self, range: Range) -> Option<TextRange> {
        self.offset(range.start)
            .and_then(|start| self.offset(range.end).map(|end| TextRange::new(start, end)))
    }

    #[must_use]
    pub fn position(&self, offset: TextSize) -> Option<Position> {
        self.offset_to_position.get(&offset).copied()
    }

    #[must_use]
    pub fn range(&self, range: TextRange) -> Option<Range> {
        self.position(range.start())
            .and_then(|start| self.position(range.end()).map(|end| Range { start, end }))
    }

    #[must_use]
    pub fn mappings(&self) -> (&BTreeMap<TextSize, Position>, &BTreeMap<Position, TextSize>) {
        (&self.offset_to_position, &self.position_to_offset)
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        self.lines
    }

    #[must_use]
    pub fn all_range(&self) -> Range {
        Range {
            start: Position {
                index: 0,
                line: 0,
                character: 0,
            },
            end: self.end,
        }
    }

    fn new_impl(source: &str, utf16: bool, base: u64) -> Self {
        let mut offset_to_position = BTreeMap::new();
        let mut position_to_offset = BTreeMap::new();

        let mut line: u64 = base;
        let mut character: u64 = base;
        let mut last_offset = 0;
        let mut index: u64 = 0;

        for (i, c) in source.chars().enumerate() {
            index = i as u64;
            let new_offset = last_offset + c.len_utf8();

            let character_size = if utf16 { c.len_utf16() } else { 1 };

            offset_to_position.extend((last_offset..new_offset).map(|b| {
                (
                    TextSize::from(b as u32),
                    Position {
                        index,
                        line,
                        character,
                    },
                )
            }));

            position_to_offset.extend((last_offset..new_offset).map(|b| {
                (
                    Position {
                        index,
                        line,
                        character,
                    },
                    TextSize::from(b as u32),
                )
            }));

            last_offset = new_offset;

            character += character_size as u64;
            if c == '\n' {
                // LF is at the start of each line.
                line += 1;
                character = base;
            }
        }

        // Last imaginary character.
        offset_to_position.insert(
            TextSize::from(last_offset as u32),
            Position {
                index,
                line,
                character,
            },
        );
        position_to_offset.insert(
            Position {
                index,
                line,
                character,
            },
            TextSize::from(last_offset as u32),
        );

        Self {
            offset_to_position,
            position_to_offset,
            lines: line as usize,
            end: Position {
                index,
                line,
                character,
            },
        }
    }
}
