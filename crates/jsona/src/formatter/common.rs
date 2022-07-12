use rowan::TextRange;
use std::iter::repeat;
use std::rc::Rc;

use super::Options;

pub(crate) fn overlaps(range: TextRange, other: TextRange) -> bool {
    range.contains_range(other)
        || other.contains_range(range)
        || range.contains(other.start())
        || range.contains(other.end())
        || other.contains(range.start())
        || other.contains(range.end())
}

#[derive(Debug, Clone)]
pub(crate) struct Context {
    pub(crate) indent_level: usize,
    pub(crate) force_multiline: bool,
    pub(crate) errors: Rc<[TextRange]>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            indent_level: Default::default(),
            force_multiline: Default::default(),
            errors: Rc::from([]),
        }
    }
}

impl Context {
    pub(crate) fn error_at(&self, range: TextRange) -> bool {
        for error_range in self.errors.iter().copied() {
            if overlaps(range, error_range) {
                return true;
            }
        }

        false
    }

    pub(crate) fn indent<'o>(&self, opts: &'o Options) -> impl Iterator<Item = &'o str> {
        repeat(opts.indent_string.as_ref()).take(self.indent_level)
    }
}
