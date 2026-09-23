use std::{collections::HashMap, io};
use dialoguer::{theme::ColorfulTheme, FuzzySelect, Select};
use parsidate::ParsiDate;
use chrono::{Local, Datelike};

#[derive(Debug, Clone)]
pub enum DateFilter {
    LastMonth(String),             // e.g. "مرداد"
    ThisMonth(String),             // e.g. "شهریور"
    Range { start: ParsiDate, end: ParsiDate },
    All,
}

/// Generates a list of all valid ParsiDate instances for current year and next year.
fn generate_dates_for_current_and_next_year(current_year: i32) -> Vec<ParsiDate> {
    let mut dates = Vec::with_capacity(366 * 2);

    for year in current_year..=current_year + 1 {
        for month in 1..=12 {
            // In the Jalali calendar:
            // Months 1..=6 have 31 days
            // Months 7..=11 have 30 days
            // Month 12 has 29 days (30 if leap year)
            let max_days = match month {
                1..=6 => 31,
                7..=11 => 30,
                12 => {
                    // Check if the year is leap using parsidate
                    if ParsiDate::is_persian_leap_year(year) {
                        30
                    } else {
                        29
                    }
                }
                _ => 0,
            };

            for day in 1..=max_days {
                if let Ok(date) = ParsiDate::new(year, month, day) {
                    dates.push(date);
                }
            }
        }
    }

    dates
}

/// Prompts the user with four options and allows date selection via FuzzySelect for Range.
pub fn select_date_option(theme:&ColorfulTheme) -> io::Result<DateFilter> {
    // let theme = ColorfulTheme::default();

    // 1. Get current date in Jalali
    let today_chrono = Local::now().date_naive();
    let today_jalali = ParsiDate::from_gregorian(
        today_chrono
    ).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

    let current_month_name = today_jalali.format("%B");

    // Calculate previous month & year
    let (last_month_num, last_month_year) = if today_jalali.month() == 1 {
        (12, today_jalali.year() - 1)
    } else {
        (today_jalali.month() - 1, today_jalali.year())
    };

    let last_month_date = ParsiDate::new(last_month_year, last_month_num, 1)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let last_month_name = last_month_date.format("%B");

    // 2. Define the main 4 menu options
    let menu_options = vec![
        format!("Last Month ({})", last_month_name),
        format!("This Month ({})", current_month_name),
        "Range".to_string(),
        "All".to_string(),
    ];

    let selection = Select::with_theme(theme)
        .with_prompt("Please select a date filter option")
        .default(0)
        .items(&menu_options)
        .interact()?;

    match selection {
        0 => Ok(DateFilter::LastMonth(last_month_name)),
        1 => Ok(DateFilter::ThisMonth(current_month_name)),
        2 => {
            // 3. Generate dates for current year and next year
            let date_pool = generate_dates_for_current_and_next_year(today_jalali.year());
            
            // Format each date as YYYY/MM/DD
            let date_strings: Vec<String> = date_pool
                .iter()
                .map(|d| format!("{:04}/{:02}/{:02}", d.year(), d.month(), d.day()))
                .collect();

            // Select Start Date
            let start_idx = FuzzySelect::with_theme(theme)
                .with_prompt("Select Start Date (Type to search YYYY/MM/DD)")
                .items(&date_strings)
                .default(0)
                .interact()?;
            let start_date = date_pool[start_idx];

            // Select End Date (starting from the chosen start date onwards)
            let end_date_pool = &date_pool[start_idx..];
            let end_date_strings = &date_strings[start_idx..];

            let end_relative_idx = FuzzySelect::with_theme(theme)
                .with_prompt("Select End Date (Type to search YYYY/MM/DD)")
                .items(end_date_strings)
                .default(0)
                .interact()?;
            let end_date = end_date_pool[end_relative_idx];

            Ok(DateFilter::Range {
                start: start_date,
                end: end_date,
            })
        }
        3 => Ok(DateFilter::All),
        _ => unreachable!(),
    }
}

// fn main() -> io::Result<()> {
//     match select_date_option()? {
//         DateFilter::LastMonth(name) => println!("Selected: Last Month -> {}", name),
//         DateFilter::ThisMonth(name) => println!("Selected: This Month -> {}", name),
//         DateFilter::Range { start, end } => {
//             println!(
//                 "Selected Range: {:04}/{:02}/{:02} to {:04}/{:02}/{:02}",
//                 start.year(), start.month(), start.day(),
//                 end.year(), end.month(), end.day()
//             );
//         }
//         DateFilter::All => println!("Selected: All"),
//     }

//     Ok(())
// }
