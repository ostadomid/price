use comfy_table::{Cell, Color};

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

impl Into<comfy_table::Row> for &Category {
    fn into(self) -> comfy_table::Row {
        comfy_table::Row::from([
            Cell::from(self.id),
            Cell::from(self.title.as_str()).fg( if self.parent.is_none() {Color::Green} else {Color::Yellow} ),
            Cell::from(self.description.as_str()),
            Cell::from(self.parent.map_or("-".to_owned(), |v|v.to_string() ) ),
        ])
    }
}

pub enum CategoryKind {
    All,
    Root,
    Child(u32),
}