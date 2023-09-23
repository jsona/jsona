macro_rules! wrap_node {
    (
    $(#[$attrs:meta])*
    $vis:vis struct $name:ident {
        inner: $inner:ident
    }
    ) => {
        $(#[$attrs])*
        $vis struct $name {
            pub(crate) inner: Rc<$inner>,
        }

        impl $crate::private::Sealed for $name {}
        impl $crate::dom::node::DomNode for $name {
            fn node_syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.node_syntax.as_ref()
            }

            fn syntax(&self) -> Option<&$crate::syntax::SyntaxElement> {
                self.inner.syntax.as_ref()
            }

            fn errors(&self) -> &$crate::util::shared::Shared<Vec<$crate::dom::error::DomError>> {
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
                    inner: Rc::new(inner)
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

    fn errors(&self) -> &Shared<Vec<DomError>> {
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
    ($elm:ident, $t:ty, $is_fn:ident, $as_fn:ident, $get_as_fn:ident) => {
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

        pub fn $get_as_fn(&self, key: impl Into<Key>) -> Option<(Key, Option<$t>)> {
            let key = key.into();
            if key.is_annotation() {
                if let Some(m) = self.annotations() {
                    for (k, v) in m.value().read().iter() {
                        if k.value() == key.value() {
                            if let Some(v) = v.$as_fn() {
                                return Some((k.clone(), Some(v.clone())));
                            } else {
                                return Some((k.clone(), None));
                            }
                        }
                    }
                }
            } else {
                if let Some(o) = self.as_object() {
                    for (k, v) in o.value().read().iter() {
                        if k.value() == key.value() {
                            if let Some(v) = v.$as_fn() {
                                return Some((k.clone(), Some(v.clone())));
                            } else {
                                return Some((k.clone(), None));
                            }
                        }
                    }
                }
            }
            None
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
