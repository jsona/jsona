use rowan::TextRange;

pub fn join_ranges<I: IntoIterator<Item = TextRange>>(ranges: I) -> TextRange {
    ranges
        .into_iter()
        .fold(None, |ranges, range| match ranges {
            Some(r) => Some(range.cover(r)),
            None => Some(range),
        })
        .unwrap()
}

pub fn try_join_ranges<I: IntoIterator<Item = TextRange>>(ranges: I) -> Option<TextRange> {
    ranges.into_iter().fold(None, |ranges, range| match ranges {
        Some(r) => Some(range.cover(r)),
        None => Some(range),
    })
}

pub fn overlaps(range: TextRange, other: TextRange) -> bool {
    range.contains_range(other)
        || other.contains_range(range)
        || range.contains(other.start())
        || range.contains(other.end())
        || other.contains(range.start())
        || other.contains(range.end())
}
