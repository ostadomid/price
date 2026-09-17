#[derive(Debug, Clone)]
pub struct Category {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub parent: Option<u32>,
}

impl Category {
    pub fn new(id: u32, title: String, description: String, parent: Option<u32>) -> Self {
        Self {
            id,
            title,
            description,
            parent,
        }
    }
}
