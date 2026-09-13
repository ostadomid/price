use chrono::NaiveDate;
use comfy_table::Cell;
use dialoguer::{Input, Select, theme::ColorfulTheme};
use format_num::format_num;
use crate::get_parsi_date;


#[derive(Debug,Default)]
pub struct Expense {
    pub id: Option<u32>,
    pub card: u32,
    pub ink: u32,
    pub printer: u32,
    pub maintanance: u32,
    pub description: String,
    pub issued_at: NaiveDate,
}

impl Expense {
    
    pub fn new_from_user(theme: &ColorfulTheme) -> Self {
        let mut new_expense = Expense::default();

        let kind = ["card","ink", "printer", "maintanance"];
        let kind = Select::with_theme(theme)
            .with_prompt("Kind")
            .items(kind)
            .default(0)
            .interact()
            .unwrap().to_owned();
        let cost = Input::<u32>::with_theme(theme)
            .with_prompt("Cost")
            .interact()
            .unwrap();
        match kind {
            0=>{new_expense.card = cost;}
            1=>{new_expense.ink = cost;}
            2=>{new_expense.printer = cost;}
            3=>{new_expense.maintanance = cost;}
            _=>{}
        }
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
            Cell::from(format_num!(",.0f", self.card)),
            Cell::from(format_num!(",.0f", self.ink)),
            Cell::from(format_num!(",.0f", self.printer)),
            Cell::from(format_num!(",.0f", self.maintanance)),
            Cell::from(self.description),
            Cell::from(
                parsidate::ParsiDate::from_gregorian(self.issued_at)
                    .unwrap()
                    .to_string(),
            ),
        ])
    }
}
