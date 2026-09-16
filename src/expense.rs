use crate::get_parsi_date;
use chrono::NaiveDate;
use comfy_table::Cell;
use dialoguer::{Input, Select, theme::ColorfulTheme};
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
        "profit-transfer"
    ];
    pub fn new_from_user(theme: &ColorfulTheme) -> Self {
        let mut new_expense = Expense::default();

        //let kind = ;
        let kind = Select::with_theme(theme)
            .with_prompt("Kind")
            .items(Self::KIND)
            .default(0)
            .interact()
            .unwrap()
            .to_owned();
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
