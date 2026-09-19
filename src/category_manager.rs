use std::collections::HashMap;

use indextree::{Arena, NodeId};

use crate::category::Category;

pub struct CategoryManager {
    pub arena: Arena<Category>,
    pub node_ids: HashMap<u32, NodeId>,
}

impl CategoryManager {
    pub fn load(categories: Vec<Category>) -> Self {
        let mut arena = Arena::new();
        let mut node_ids = HashMap::new();
        let mut child_parent = Vec::<(u32, u32)>::with_capacity(categories.len());

        for category in categories {
            let id = category.id;
            if let Some(parent) = category.parent {
                child_parent.push((id, parent));
            }
            let node_id = arena.new_node(category);
            node_ids.entry(id).or_insert(node_id);
        }
        for (child, parent) in child_parent {
            match (node_ids.get(&child), node_ids.get(&parent)) {
                (Some(child), Some(mut parent)) => {
                    parent.append(*child, &mut arena);
                }
                _ => {}
            }
        }
        Self { arena, node_ids }
    }
}
