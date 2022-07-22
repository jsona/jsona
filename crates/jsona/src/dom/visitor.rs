use super::{DomNode, Keys, Node};

pub struct Visitor<'a> {
    node: &'a Node,
    f: VisitFn,
}

type VisitFn = Box<dyn Fn(&Keys, &Node) -> (bool, bool)>;

impl<'a> Visitor<'a> {
    pub fn new(node: &'a Node, f: impl Fn(&Keys, &Node) -> (bool, bool) + 'static) -> Self {
        Self {
            node,
            f: Box::new(f),
        }
    }
}

impl<'a> IntoIterator for Visitor<'a> {
    type Item = (Keys, Node);
    type IntoIter = std::vec::IntoIter<(Keys, Node)>;

    fn into_iter(self) -> Self::IntoIter {
        fn collect(keys: Keys, node: &Node, all: &mut Vec<(Keys, Node)>, f: &VisitFn) {
            let (add, iter) = f(&keys, node);
            if add {
                all.push((keys.clone(), node.clone()));
            }
            if !iter {
                return;
            }
            match node {
                Node::Object(obj) => {
                    let props = obj.inner.properties.read();
                    for (key, node) in &props.all {
                        collect(keys.join(key.into()), node, all, f);
                    }
                }
                Node::Array(arr) => {
                    let items = arr.inner.items.read();
                    for (idx, node) in items.iter().enumerate() {
                        collect(keys.join(idx.into()), node, all, f);
                    }
                }
                _ => {}
            }

            if let Some(annotations) = node.annotations() {
                let members = annotations.value().read();
                for (key, node) in &members.all {
                    collect(keys.join(key.into()), node, all, f);
                }
            }
        }

        let mut all = vec![];
        collect(Keys::default(), self.node, &mut all, &self.f);

        all.into_iter()
    }
}

pub fn visit_annotations(node: &Node) -> Visitor {
    Visitor::new(node, |keys, _| {
        if keys
            .last()
            .map(|v| v.is_annotation_key())
            .unwrap_or_default()
        {
            (true, false)
        } else {
            (false, true)
        }
    })
}
