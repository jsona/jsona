use super::{DomNode, Keys, Node};

type VisitFn<'a, T> = Box<dyn Fn(&Keys, &Node, &T) -> VisitControl + 'a>;

pub struct Visitor<'a, T> {
    node: &'a Node,
    state: &'a T,
    f: VisitFn<'a, T>,
}

impl<'a, T> Visitor<'a, T> {
    pub fn new(
        node: &'a Node,
        state: &'a T,
        f: impl Fn(&Keys, &Node, &T) -> VisitControl + 'a,
    ) -> Self {
        Self {
            node,
            state,
            f: Box::new(f),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisitControl {
    AddIter,
    NotAddIter,
    AddNotIter,
    NotAddNotIter,
}

impl<'a, T> IntoIterator for Visitor<'a, T> {
    type Item = (Keys, Node);
    type IntoIter = std::vec::IntoIter<(Keys, Node)>;

    fn into_iter(self) -> Self::IntoIter {
        let mut all = vec![];
        collect(Keys::default(), self.node, self.state, &mut all, &self.f);
        all.into_iter()
    }
}

fn collect<T>(
    keys: Keys,
    node: &Node,
    state: &T,
    all: &mut Vec<(Keys, Node)>,
    f: &dyn Fn(&Keys, &Node, &T) -> VisitControl,
) {
    match f(&keys, node, state) {
        VisitControl::AddIter => {
            all.push((keys.clone(), node.clone()));
        }
        VisitControl::NotAddIter => {}
        VisitControl::AddNotIter => {
            all.push((keys.clone(), node.clone()));
            return;
        }
        VisitControl::NotAddNotIter => {
            return;
        }
    }
    match node {
        Node::Object(obj) => {
            let props = obj.inner.properties.read();
            for (key, node) in props.iter() {
                collect(keys.join(key.into()), node, state, all, &f);
            }
        }
        Node::Array(arr) => {
            let items = arr.inner.items.read();
            for (idx, node) in items.iter().enumerate() {
                collect(keys.join(idx.into()), node, state, all, &f);
            }
        }
        _ => {}
    }

    if let Some(annotations) = node.annotations() {
        let map = annotations.value().read();
        for (key, node) in map.iter() {
            collect(keys.join(key.into()), node, state, all, &f);
        }
    }
}

pub fn visit_annotations(node: &Node) -> Visitor<()> {
    Visitor::new(node, &(), |keys, _, _| {
        if keys
            .last()
            .map(|v| v.is_annotation_key())
            .unwrap_or_default()
        {
            VisitControl::AddNotIter
        } else {
            VisitControl::NotAddIter
        }
    })
}
