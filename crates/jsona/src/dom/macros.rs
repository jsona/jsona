macro_rules! wrap_node {
    (
    $(#[$attrs:meta])*
    $vis:vis struct $name:ident {
        inner: $inner:ident
    }
    ) => {
        $(#[$attrs])*
        $vis struct $name {
            pub(crate) inner: Arc<$inner>,
        }

        impl $crate::private::Sealed for $name {}
        impl $crate::dom::node::DomNode for $name {
            fn node_syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.node_syntax.as_ref()
            }

            fn syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.syntax.as_ref()
            }

            fn errors(&self) -> &$crate::util::shared::Shared<Vec<$crate::dom::error::Error>> {
                &self.inner.errors
            }

            fn annotations(&self) -> Option<&$crate::dom::node::Annotations> {
                self.inner.annotations.as_ref()
            }
        }

        impl $inner {
            #[allow(dead_code)]
            pub(crate) fn into_node(self) -> $crate::dom::Node {
				$name::from(self).into()
            }
        }

        impl From<$inner> for $name {
            fn from(inner: $inner) -> $name {
                $name {
                    inner: Arc::new(inner)
                }
            }
        }
    };
}

macro_rules! impl_dom_node_for_node {
    (
        $(
          $elm:ident,
        )*
    ) => {
impl DomNode for Node {
    fn node_syntax(&self) -> Option<&SyntaxElement> {
        match self {
            $(
            Node::$elm(v) => v.node_syntax(),
            )*
        }
    }

    fn syntax(&self) -> Option<&SyntaxElement> {
        match self {
            $(
            Node::$elm(v) => v.syntax(),
            )*
        }
    }

    fn errors(&self) -> &Shared<Vec<Error>> {
        match self {
            $(
            Node::$elm(v) => v.errors(),
            )*
        }
    }

    fn annotations(&self) -> Option<&Annotations> {
        match self {
            $(
            Node::$elm(v) => v.annotations(),
            )*
        }
    }
}
    };
}

macro_rules! define_value_fns {
    ($elm:ident, $t:ty, $is_fn:ident, $as_fn:ident, $try_get_as_fn:ident) => {
        pub fn $is_fn(&self) -> bool {
            matches!(self, Self::$elm(..))
        }

        pub fn $as_fn(&self) -> Option<&$t> {
            if let Self::$elm(v) = self {
                Some(v)
            } else {
                None
            }
        }

        pub fn $try_get_as_fn(&self, key: &KeyOrIndex) -> Result<Option<$t>, KeyError> {
            match self.get(key) {
                None => Ok(None),
                Some(v) => {
                    if let Node::$elm(v) = v {
                        Ok(Some(v))
                    } else {
                        Err(KeyError::UnexpectedType)
                    }
                }
            }
        }
    };
}

macro_rules! value_from {
    (
        $(
          $elm:ident,
        )*
    ) => {
    $(
    impl From<$elm> for Node {
        fn from(v: $elm) -> Self {
            Self::$elm(v)
        }
    }
    )*
    };
}
