use super::Octree;

use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize)]
pub enum OctreeData<E> {
    Node([Arc<Octree<E>>; 8]),
    Leaf(Arc<E>),
    Empty,
}

use OctreeData::*;
impl<E: PartialEq> PartialEq for OctreeData<E> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Empty, Empty) => true,
            (Leaf(ref a_elem), Leaf(ref b_elem)) => *a_elem == *b_elem,
            (Node(ref a_nodes), Node(ref b_nodes)) => *a_nodes == *b_nodes,
            (_, _) => false,
        }
    }
}
impl<E: Eq> Eq for OctreeData<E> {}

impl<E> Clone for OctreeData<E> {
    fn clone(&self) -> Self {
        match *self {
            Node(ref nodes) => Node(nodes.clone()),
            Leaf(ref arc_elem) => Leaf(arc_elem.clone()),
            Empty => Empty,
        }
    }
}

impl<E> OctreeData<E> {
    pub fn is_node(&self) -> bool {
        match self {
            Node(_) => true,
            _ => false,
        }
    }
}
