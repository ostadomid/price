use std::error::Error;

use chrono::NaiveDate;
use dialoguer::{Select, theme::ColorfulTheme};

macro_rules! hashmap {
    ( $($key:expr=>$value:expr),* $(,)? ) => {
        {
            let mut _h = std::collections::HashMap::new();
            $(_h.insert($key,$value);)*
            _h
        }
    };
}

pub const MONTHS: [&str; 12] = [
    "Farvardin",
    "Ordibehesht",
    "Khordad",
    "Tir",
    "Mordad",
    "Shahrivar",
    "Mehr",
    "Aban",
    "Azar",
    "Dey",
    "Bahman",
    "Esfand",
];

#[derive(Debug, Clone)]
pub struct DateRange {
    start: NaiveDate,
    end: NaiveDate,
}

pub fn generate_dates() -> Result<Vec<String>, Box<dyn Error>> {
    let mut result = Vec::<String>::with_capacity(366 * 2);
    let today = parsidate::ParsiDate::today()?;
    let year = today.year();
    for y in year..=year + 1 {
        for m in 1..=12 {
            for d in 1..=31 {
                let instance = unsafe { parsidate::ParsiDate::new_unchecked(y, m, d) };
                if instance.is_valid() {
                    result.push(instance.format("short"));
                }
            }
        }
    }
    Ok(result)
}
fn prev_month() -> &'static str {
    let month = parsidate::ParsiDate::today()
        .expect("msg")
        .sub_months(1)
        .expect("msg")
        .month() as usize
        - 1;
    MONTHS[month]
}
fn current_month() -> &'static str {
    let month = parsidate::ParsiDate::today().expect("msg").month() as usize - 1;
    MONTHS[month]
}

impl DateRange {
    pub fn new_from_ui(theme: &ColorfulTheme) -> Self {
        let dates = generate_dates().expect("Can not generate dates");
        let menu_items = [
            "All",
            prev_month(),
            current_month(),
            "Range",
        ];
        let beginning = parsidate::ParsiDate::new(1405, 5, 1).unwrap();
        let today = parsidate::ParsiDate::today().unwrap();
        match Select::with_theme(theme)
            .items(menu_items)
            .interact()
            .unwrap()
        {
            0 => Self {
                start: beginning.to_gregorian().unwrap(),
                end: today.to_gregorian().unwrap(),
            },
            1 => Self {
                start: today
                    .sub_months(1)
                    .unwrap()
                    .first_day_of_month()
                    .to_gregorian()
                    .unwrap(),
                end: today
                    .sub_months(1)
                    .unwrap()
                    .last_day_of_month()
                    .to_gregorian()
                    .unwrap(),
            },
            2 => Self {
                start: today.first_day_of_month().to_gregorian().unwrap(),
                end: today.last_day_of_month().to_gregorian().unwrap(),
            },
            3 => {
                unimplemented!()
            }
            _ => unimplemented!(),
        }
    }
}
