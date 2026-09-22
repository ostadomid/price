use comfy_table::{
    Cell,
    Color::{Blue, Green, Red, White, Yellow},
    Row,
};

pub struct InOutReport {
    title: String,
    pub v1: u32,
    pub v2: u32,
    pub v3: u32,
    pub v4: u32,
}
impl InOutReport {
    pub fn new(title: String, v1: u32, v2: u32, v3: u32, v4: u32) -> Self {
        Self {
            title,
            v1, /* card */
            v2, /* ink-printer */
            v3, /* profit */
            v4, /* balance */
        }
    }
    // pub fn car_plus_ink_plus_printer(&self)->u32{
    //     self.v1 + self.v2
    // }
    pub fn summary(income: &Self, outcome: &Self) -> Self {
        Self {
            title: "Summary".into(),
            v1: income.v1.saturating_sub(outcome.v1) ,
            v2: income.v2.saturating_sub(outcome.v2),
            v3: income.v3.saturating_sub(outcome.v3),
            v4: income.v4 - (/*income.v3 + */ outcome.v1 + outcome.v2 + outcome.v3), // موجودی فعلی حساب را منهای هزینه های کارت و جوهر و پرینتر و سود میکنیم تا مانده حساب پیدا شود
        }
    }
}

impl Into<Row> for InOutReport {
    fn into(self) -> comfy_table::Row {
        let color = match self.title.to_lowercase().as_ref() {
            "+" => Yellow,
            "-" => Red,
            "summary" => Green,
            _ => White,
        };
        Row::from(vec![
            Cell::from(self.title.as_str()),
            Cell::new(format_num::format_num!(",.0f", self.v1).as_str()).fg(color),
            Cell::new(format_num::format_num!(",.0f", self.v2).as_str()).fg(color),
            Cell::new(format_num::format_num!(",.0f", self.v3).as_str()).fg(color),
            Cell::new(format_num::format_num!(",.0f", self.v4).as_str()).fg(color),
        ])
    }
}
