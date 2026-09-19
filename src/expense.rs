use crate::{category_manager::CategoryManager, get_categories, get_parsi_date};
use chrono::NaiveDate;
use comfy_table::Cell;
use dialoguer::{FuzzySelect, Input, Select, theme::ColorfulTheme};
use format_num::format_num;

#[derive(Debug, Default)]
pub struct Expense {
    pub id: Option<u32>,
    pub kind: String,
    pub cost: u32,
    pub description: String,
    pub issued_at: NaiveDate,
}

impl Expense {
    pub const KIND: [&str; 14] = [
        "card",
        "ink",
        "printer",
        "maintanance",
        "kami-payment",
        "bill-electricity",
        "bill-water",
        "bill-phone",
        "pharmacy",
        "meat",
        "supermarket",
        "printhouse",
        "insurance",
        "profit-transfer",
    ];
    pub fn new_from_user(theme: &ColorfulTheme) -> Self {
        let mut new_expense = Expense::default();
        let cm = CategoryManager::load(get_categories(crate::category::CategoryKind::All));
        let mut categories = Vec::<(String, String)>::new();
        for key in cm.node_ids.keys() {
            if let Some(&node_id) = cm.node_ids.get(key) {
                if !node_id.is_leaf(&cm.arena) {
                    continue;
                }
                let ancestors = node_id.ancestors(&cm.arena).collect::<Vec<_>>();
                let parts = ancestors
                    .iter()
                    .rev()
                    .map(|r| cm.arena.get(*r).unwrap().get().title.as_str())
                    .collect::<Vec<&str>>();
                categories.push((parts.join(" -> "), parts.last().unwrap().to_string()));
            }
        }
        categories.sort_by_key(|e| e.0.clone());
        //let kind = ;
        let category_idx = FuzzySelect::with_theme(theme)
            .with_prompt("Kind")
            .items(categories.iter().map(|c| c.0.as_str()))
            .default(0)
            .interact()
            .unwrap();

        let kind = categories[category_idx].1.clone();
        
        new_expense.kind = kind;
        let cost = Input::<u32>::with_theme(theme)
            .with_prompt("Cost")
            .interact()
            .unwrap();
        new_expense.cost = cost;

        new_expense.description = Input::<String>::with_theme(theme)
            .with_prompt("Description")
            .default("".to_owned())
            .interact()
            .unwrap();
        new_expense.issued_at = get_parsi_date(theme).to_gregorian().unwrap();

        new_expense
    }
}
impl Into<comfy_table::Row> for Expense {
    fn into(self) -> comfy_table::Row {
        comfy_table::Row::from(vec![
            Cell::from(self.id.unwrap_or_default().to_string()),
            Cell::from(self.kind),
            Cell::from(format_num!(",.0f", self.cost)),
            Cell::from(self.description),
            Cell::from(
                parsidate::ParsiDate::from_gregorian(self.issued_at)
                    .unwrap()
                    .to_string(),
            ),
        ])
    }
}
