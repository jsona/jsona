use super::node::Key;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyOrIndex {
    Index(usize),
    Key(Key),
    AnnoKey(Key),
}

impl<N> From<N> for KeyOrIndex
where
    N: Into<usize>,
{
    fn from(v: N) -> Self {
        Self::Index(v.into())
    }
}

impl core::fmt::Display for KeyOrIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeyOrIndex::Index(v) => v.fmt(f),
            KeyOrIndex::Key(v) => v.fmt(f),
            KeyOrIndex::AnnoKey(v) => write!(f, "@{}", v),
        }
    }
}

impl KeyOrIndex {
    pub fn new_key(k: Key) -> Self {
        Self::Key(k)
    }

    pub fn new_anno_key(k: Key) -> Self {
        Self::AnnoKey(k)
    }

    /// Returns `true` if the key or index is [`Index`].
    ///
    /// [`Index`]: KeyOrIndex::Index
    pub fn is_index(&self) -> bool {
        matches!(self, Self::Index(..))
    }

    /// Returns `true` if the key or index is [`Key`].
    ///
    /// [`Key`]: KeyOrIndex::Key
    pub fn is_key(&self) -> bool {
        matches!(self, Self::Key(..))
    }

    /// Returns `true` if the key or index is [`AnnoKey`].
    ///
    /// [`Key`]: KeyOrIndex::Key
    pub fn is_anno_key(&self) -> bool {
        matches!(self, Self::AnnoKey(..))
    }

    pub fn as_index(&self) -> Option<&usize> {
        if let Self::Index(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_key(&self) -> Option<&Key> {
        if let Self::Key(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_anno_key(&self) -> Option<&Key> {
        if let Self::AnnoKey(v) = self {
            Some(v)
        } else {
            None
        }
    }
}
