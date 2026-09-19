#[cfg(test)]
pub mod tests {
    #[test]
    #[allow(dead_code, unused)]
    pub fn one() {
        struct Category {
            id: u32,
            title: String,
        }
        impl Category {
            fn new(id: u32, title: &str) -> Self {
                Self {
                    id,
                    title: title.into(),
                }
            }
        }
        // let mut arena = &mut indextree::Arena::<Category>::new();
        // let mut a = arena.new_node(Category::new(1, "bill"));
        // let mut b = arena.new_node(Category::new(1, "water"));
        // let mut c = arena.new_node(Category::new(1, "gas"));
        // a.append(b, arena);
        // a.append(c, arena);
        // let ancestors = b.ancestors(arena).collect::<Vec<_>>();
        // let bread = ancestors
        //     .iter()
        //     .rev()
        //     .map(|e| arena[*e].get().title.as_str())
        //     .copied();
        //   bread.
            
            
        // println!("{bread}");
        // for nid in ancestors {
        //     println!("{}", arena[nid].get().title);
        // }
        // assert_eq!(a.child_count(arena), 2);
    }
}
