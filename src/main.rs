#![allow(dead_code, unused)]
mod charts;
mod expense;
mod report;
mod sql;
use std::{
    collections::HashMap,
    env,
    error::Error,
    fmt::Display,
    fs::File,
    io::{BufReader, BufWriter},
    sync::{LazyLock, Mutex, OnceLock},
};

use chrono::NaiveDate;
use colored::*;
use comfy_table::{Cell, Color, Row, Table};
use dialoguer::{Confirm, FuzzySelect, Input, Select, console::Style, theme::ColorfulTheme};
use format_num::format_num;
use num_format::{
    Buffer, Format,
    Locale::{self, shi},
    ToFormattedString,
};
use parsidate::ParsiDate;
use rusqlite::{Connection, params};
use rust_xlsxwriter::workbook::Workbook;
use serde::{Deserialize, Serialize};
use tempfile::{Builder, tempfile};

use crate::{
    expense::Expense,
    report::InOutReport,
    sql::{CREATE_EXPENSE_TABLE, INSERT_EXPENSE, ORDERS_EXPENSE_REPORT, SHOW_EXPENSES},
};

static DB_CONNECTION: OnceLock<Mutex<Connection>> = OnceLock::new();
const MONTHS: [&str; 12] = [
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
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
enum Shipment {
    Almas(u32),
    Hamrang(u32),
    Inventory(u32),
}
impl Shipment {
    fn prefix(&self) -> String {
        match self {
            Shipment::Almas(_) => "AL-".into(),
            Shipment::Hamrang(_) => "H-".into(),
            _ => "".into(),
        }
    }
    fn to_string(&self) -> String {
        match self {
            Shipment::Almas(_) => "Almas".into(),
            Shipment::Hamrang(_) => "Hamrang".into(),
            Shipment::Inventory(_) => "Inventory".into(),
        }
    }
    fn value(&self) -> u32 {
        match self {
            Shipment::Almas(v) | Shipment::Hamrang(v) | Shipment::Inventory(v) => *v,
            _ => 0,
        }
    }
}
impl Display for Shipment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Shipment::Almas(_) => "Almas",
                Shipment::Hamrang(_) => "Hamrang",
                Shipment::Inventory(_) => "Inventory",
            }
        )
    }
}
#[derive(Debug, Serialize, Deserialize, Clone)]
struct Config {
    printer_price: u32,
    total_prints: u32,
    ink_price: u32,
    shipment_cost: Vec<Shipment>,
    profit_margin: f64,
}
impl Display for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Config {{\n\tPrinter Price: {}\n\tInk Price: {}\n\tPrinter Renewal Counter: {}\n}}",
            self.printer_price.to_formatted_string(&Locale::en),
            self.ink_price.to_formatted_string(&Locale::en),
            self.total_prints.to_formatted_string(&Locale::en),
        )
    }
}
#[derive(Debug, Default)]
struct Order {
    id: Option<u32>,
    card_id: String,
    order_count: u32,
    card_raw_price: u32,
    card_cost: f64,
    card_profit: f64,
    design_cost: u32,
    discount: u32,
    customer_paid: f64,
    ordered_at: Option<NaiveDate>,
}
impl Order {
    fn new_from_user(
        config: &Config,
        theme: &ColorfulTheme,
        prices: &HashMap<String, Vec<u32>>,
        raw_prices: &HashMap<String, u32>,
    ) -> Self {
        let cost_printer_usage = config.printer_price / config.total_prints;
        let cost_ink = config.ink_price / config.total_prints;
        let keys = prices.keys().cloned().collect::<Vec<String>>();

        let card_idx = FuzzySelect::with_theme(theme)
            .with_prompt("Card")
            .items(&keys)
            .max_length(7)
            .interact()
            .unwrap();
        let card_id = &keys[card_idx];
        let card_raw_price = *raw_prices.get(card_id).unwrap_or(&0);
        let card_prices = prices.get(card_id).unwrap();
        let card_final_price = card_prices[Select::with_theme(theme)
            .with_prompt("Card Price")
            .items(
                card_prices
                    .iter()
                    .map(|e| format_num!(",.0f", *e))
                    .collect::<Vec<String>>(),
            )
            .default(0)
            .interact()
            .unwrap()];
        let order_count = Input::<u32>::with_theme(theme)
            .with_prompt("Order Count")
            .default(1)
            .interact_text()
            .unwrap();
        let mut d_cost = vec![];
        d_cost.push(if order_count <= 200 {
            400_000
        } else {
            order_count * 2_000
        });
        d_cost.append(&mut vec![order_count * 2_000, 0]);

        let mut design_cost = d_cost[Select::with_theme(theme)
            .with_prompt("Design Cost")
            .items(vec![
                format!("Normal - {}", format_num!(",.0f", d_cost[0])).as_str(),
                format!("Re-Order {}", format_num!(",.0f", d_cost[1])).as_str(),
                "Others",
            ])
            .default(0)
            .interact()
            .unwrap()];
        if design_cost == 0 {
            design_cost = Input::<u32>::with_theme(theme)
                .with_prompt("Custom Design Cost")
                .default(0)
                .interact()
                .unwrap_or_default()
        }
        // let design_cost = Input::<u32>::with_theme(theme)
        //     .with_prompt("Design Cost")
        //     .default(if order_count < 200 {
        //         400_000
        //     } else {
        //         order_count * 2000
        //     })
        //     .interact_text()
        //     .unwrap_or_default();
        let discount = Input::<u32>::with_theme(theme)
            .with_prompt("Discount")
            .default(0)
            .interact_text()
            .unwrap_or_default();
        let ordered_at = get_parsi_date(theme);
        let ordered_at = ordered_at.to_gregorian().unwrap();

        let card_cost = (card_final_price as f64) / (1.0 + config.profit_margin);
        let card_profit = card_cost * config.profit_margin;

        let order_profit = order_count as f64 * card_profit + design_cost as f64 - discount as f64;
        let customer_paid = order_count as f64 * card_cost * (1.0 + config.profit_margin)
            + design_cost as f64
            - discount as f64;
        Self {
            id: None,
            card_id: card_id.to_owned(),
            order_count,
            card_raw_price,
            card_cost,
            card_profit,
            design_cost,
            discount,
            customer_paid,
            ordered_at: Some(ordered_at),
        }
    }
}

impl Into<comfy_table::Row> for &Order {
    fn into(self) -> comfy_table::Row {
        comfy_table::Row::from(vec![
            Cell::from(self.id.unwrap_or_default().to_string()),
            Cell::from(self.card_id.to_string()),
            Cell::from(self.order_count.to_string()),
            Cell::from(format_num!(",.0f", self.card_raw_price)),
            Cell::from(format_num!(",.0f", self.card_cost)),
            Cell::from(format_num!(",.0f", self.card_profit)).fg(Color::Cyan),
            Cell::from(format_num!(",.0f", self.design_cost)),
            Cell::from(format_num!(",.0f", self.discount)),
            Cell::from(format_num!(",.0f", self.customer_paid)),
            Cell::from(
                parsidate::ParsiDate::from_gregorian(self.ordered_at.unwrap_or_default())
                    .unwrap()
                    .to_string(),
            ),
        ])
    }
}
fn read_config(path: &str) -> std::io::Result<Config> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let config: Config = serde_json::from_reader(reader)?;
    Ok(config)
}
fn write_config(config: &Config, path: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, config)?;
    Ok(())
}
fn calculate_final_price(config: &Config, theme: &ColorfulTheme) {
    let cost_printer_usage = config.printer_price / config.total_prints;
    let cost_ink = config.ink_price / config.total_prints;
    'outer: loop {
        let selected_shipment = Select::with_theme(theme)
            .with_prompt("Shipment")
            .items(["Almas", "Hamrang", "Inventory", "Back"])
            .default(0)
            .interact()
            .unwrap();
        let cost_shipment = config
            .shipment_cost
            .iter()
            .find_map(|shipment| match (selected_shipment, shipment) {
                (0, Shipment::Almas(p)) => Some(*p),
                (1, Shipment::Hamrang(p)) => Some(*p),
                (2, Shipment::Inventory(p)) => Some(*p),
                (3, _) => Some(0),
                _ => None,
            })
            .unwrap_or_default();
        if cost_shipment == 0 {
            return;
        }
        let cost_shipment = Input::<u32>::with_theme(theme)
            .with_prompt("Shipment Price")
            .default(cost_shipment)
            .interact()
            .unwrap();
        'inner: loop {
            let card_raw_price = Input::<u32>::with_theme(theme)
                .with_prompt("Card Raw Price")
                .interact_text()
                .unwrap_or_default();

            // println!(
            //     "\nCost Printer Usage: {}",
            //     cost_printer_usage.to_string().cyan()
            // );
            // println!("Cost Ink: {}", cost_ink.to_string().cyan());
            // println!("Cost Shipment: {}", cost_shipment.to_string().cyan());
            // println!("Margin: {}", 1.0 + config.profit_margin);
            let final_price = ((card_raw_price as f64 * 1.05)
                + (cost_printer_usage + cost_ink + cost_shipment) as f64)
                * (1.0 + config.profit_margin);

            let mut table = Table::new();

            table.add_row(vec![
                Cell::new("Final Price"),
                Cell::new(format_num!(",.2f", final_price)).fg(Color::Yellow),
            ]);
            println!("{table}");

            let choose = Select::with_theme(theme)
                .with_prompt("Continue with current shipment?")
                .items(["Yes", "No"])
                .default(0)
                .interact()
                .unwrap();
            if choose == 1 {
                break 'inner;
            }
        }
    }
}
fn read_prices() -> std::io::Result<HashMap<String, Vec<u32>>> {
    let file = File::open("./prices.json")?;
    let mut buffer = BufReader::new(file);
    let hm: HashMap<String, Vec<u32>> = serde_json::from_reader(buffer)?;
    Ok(hm)
}
fn read_raw_prices() -> std::io::Result<HashMap<String, u32>> {
    let file = File::open("./raw-prices.json")?;
    let mut buffer = BufReader::new(file);
    let hm: HashMap<String, u32> = serde_json::from_reader(buffer)?;
    Ok(hm)
}
fn get_prices_from_raw_prices(config: &Config) -> HashMap<String, Vec<u32>> {
    let raw_prices = read_raw_prices().unwrap();

    let shipment_for_inventory = config
        .shipment_cost
        .iter()
        .find_map(|sh| {
            if sh.prefix() == "" {
                Some(sh.value())
            } else {
                None
            }
        })
        .unwrap();
    let result = raw_prices
        .iter()
        .map(|(card_code, raw_price)| {
            let prefix = &card_code[0..=card_code.find(|c| c == '-').unwrap()];
            let shipment = config
                .shipment_cost
                .iter()
                .find(|sh| sh.prefix().as_str() == prefix);

            let shipment_price = if let Some(sh) = shipment {
                sh.value()
            } else {
                shipment_for_inventory
            };
            let cost_printer_usage = config.printer_price / config.total_prints;
            let cost_ink = config.ink_price / config.total_prints;

            let lower_price = ((*raw_price as f64 * 1.05
                + (cost_ink + cost_printer_usage + shipment_for_inventory) as f64)
                * (1.0 + config.profit_margin)) as u32;
            let higher_price = ((*raw_price as f64 * 1.05
                + (cost_ink + cost_printer_usage + shipment_price) as f64)
                * (1.0 + config.profit_margin)) as u32;
            let lower_price = if lower_price % 100 < 30 {
                lower_price / 100 * 100
            } else {
                lower_price / 100 * 100 + 100
            };
            let higher_price = if higher_price % 100 < 30 {
                higher_price / 100 * 100
            } else {
                higher_price / 100 * 100 + 100
            };

            (
                card_code.clone(),
                if lower_price == higher_price {
                    vec![lower_price]
                } else {
                    vec![lower_price, higher_price]
                },
            )
        })
        .collect::<HashMap<String, Vec<u32>>>();
    result
}
fn init_db() -> Result<(), Box<dyn Error>> {
    let c = Connection::open("./databse.sqlite").expect("Cant find/create database.sqlite");
    c.execute(sql::CREATE_ORDER_TABLE, [])
        .expect("Cant create orders table");
    c.execute(CREATE_EXPENSE_TABLE, [])
        .expect("Cant create expenses table ");
    let _ = DB_CONNECTION.set(Mutex::new(c));
    Ok(())
}
fn get_db_connection() -> &'static Mutex<Connection> {
    DB_CONNECTION.get().unwrap()
}
fn create_ui_table_from_all_orders(orders: &Vec<Order>) -> Table {
    let mut table = Table::new();
    table.set_header(Row::from([
        "Id",
        "Code",
        "Count",
        "Raw Price",
        "Cost",
        "Profit",
        "Design",
        "Discount",
        "Paid",
        "Date",
    ]));
    for order in orders {
        table.add_row(order);
    }
    table
}
fn create_ui_table_from_order(order: &Order) -> Table {
    let mut table = Table::new();
    table.add_row(vec![
        "Count",
        format_num!(",.0f", order.order_count).as_str(),
    ]);
    table.add_row(vec![
        "One Card Cost",
        format_num!(",.0f", order.card_cost).as_str(),
    ]);
    table.add_row(vec![
        Cell::new("One Card Profit"),
        Cell::new(format_num!(",.0f", order.card_profit)).fg(Color::Green),
    ]);
    table.add_row(vec![
        "Card Final Price",
        format_num!(",.0f", order.card_cost + order.card_profit).as_str(),
    ]);
    table.add_row(vec![
        "Customer Paid",
        format_num!(",.0f", order.customer_paid).as_str(),
    ]);

    table.add_row(vec![
        Cell::new("Order Profit"),
        Cell::new(format_num!(
            ",.0f",
            order.order_count as f64 * order.card_profit + order.design_cost as f64
                - order.discount as f64
        ))
        .fg(Color::Green),
    ]);
    table
}

fn calculate_profit2(config: &Config, theme: &ColorfulTheme, prices: &HashMap<String, Vec<u32>>) {
    let cost_printer_usage = config.printer_price / config.total_prints;
    let cost_ink = config.ink_price / config.total_prints;
    let keys = prices.keys().cloned().collect::<Vec<String>>();
    loop {
        let card_idx = FuzzySelect::with_theme(theme)
            .with_prompt("Card")
            .items(&keys)
            .max_length(7)
            .interact()
            .unwrap();
        let card = &keys[card_idx];
        let card_prices = prices.get(card).unwrap();
        let card_final_price = card_prices[Select::with_theme(theme)
            .items(
                card_prices
                    .iter()
                    .map(|e| format_num!(",.0f", *e))
                    .collect::<Vec<String>>(),
            )
            .default(0)
            .interact()
            .unwrap()];
        let count = Input::<u32>::with_theme(theme)
            .with_prompt("Order Count")
            .interact_text()
            .unwrap();

        let design_cost = Input::<u32>::with_theme(theme)
            .with_prompt("Design Cost")
            .default(if count < 200 { 400_000 } else { count * 2000 })
            .interact_text()
            .unwrap_or_default();
        let discount = Input::<u32>::with_theme(theme)
            .with_prompt("Discount")
            .default(0)
            .interact_text()
            .unwrap_or_default();

        let one_card_cost = (card_final_price as f64) / (1.0 + config.profit_margin);
        let one_card_profit = one_card_cost * config.profit_margin;

        let order_profit = count as f64 * one_card_profit + design_cost as f64 - discount as f64;
        let customer_paid = count as f64 * one_card_cost * (1.0 + config.profit_margin)
            + design_cost as f64
            - discount as f64;
        // println!(
        //     "Card Final Price={card_final_price}\nOne Card Cost={one_card_cost}\nOne Card Profit={one_card_profit}\nCustomer Paid={customer_paid}"
        // );

        let mut table = Table::new();
        table.add_row(vec!["Count", format_num!(",.0f", count).as_str()]);
        table.add_row(vec![
            "One Card Cost",
            format_num!(",.0f", one_card_cost).as_str(),
        ]);
        table.add_row(vec![
            Cell::new("One Card Profit"),
            Cell::new(format_num!(",.0f", one_card_profit)).fg(Color::Green),
        ]);
        table.add_row(vec![
            "Card Final Price",
            format_num!(",.0f", card_final_price).as_str(),
        ]);
        table.add_row(vec![
            "Customer Paid",
            format_num!(",.0f", customer_paid).as_str(),
        ]);

        table.add_row(vec![
            Cell::new("Order Profit"),
            Cell::new(format_num!(",.0f", order_profit)).fg(Color::Green),
        ]);
        println!("{table}");
        match Select::with_theme(theme)
            .with_prompt("Continue?")
            .items(["Yes", "No"])
            .default(0)
            .interact()
            .unwrap()
        {
            0 => {}
            _ => {
                break;
            }
        }
    }
}
fn generate_dates() -> Vec<String> {
    let mut result = Vec::<String>::with_capacity(365 * 2);
    let current_year = parsidate::ParsiDate::today().unwrap().year();
    for year in current_year..current_year + 2 {
        for month in 1..=12 {
            for day in 1..=31 {
                if month >= 7 && day == 31 {
                    continue;
                }
                result.push(format!("{}/{:02}/{:02}", year, month, day));
            }
        }
    }
    result
}
fn get_parsi_date(theme: &ColorfulTheme) -> parsidate::ParsiDate {
    let items = generate_dates();
    let idx = FuzzySelect::with_theme(theme)
        .with_prompt("Select Date")
        .items(&items)
        .max_length(4)
        .interact()
        .unwrap();
    parsidate::ParsiDate::parse(items[idx].as_str(), "%Y/%m/%d").unwrap()
}

fn show_orders(theme: &ColorfulTheme) {
    match Select::with_theme(theme)
        .with_prompt("Select")
        .items(["Current Month", "Range", "Specified Card"])
        .default(0)
        .interact()
        .unwrap()
    {
        0 => {
            let start = parsidate::ParsiDate::today()
                .unwrap()
                .first_day_of_month()
                .to_gregorian()
                .unwrap();
            let end = parsidate::ParsiDate::today()
                .unwrap()
                .last_day_of_month()
                .to_gregorian()
                .unwrap();
            show_orders_table(start, end);
        }
        1 => {
            let (start, end) = (
                get_parsi_date(theme).to_gregorian().unwrap(),
                get_parsi_date(theme).to_gregorian().unwrap(),
            );
            show_orders_table(start, end);
        }
        2 => {
            show_orders_for_specific_card(theme);
        }
        _ => unimplemented!(),
    }
}
fn show_orders_for_specific_card(theme: &ColorfulTheme) {}
fn show_orders_table(start: NaiveDate, end: NaiveDate) {
    println!("{}-{}", start, end);
    let db = get_db_connection().lock().unwrap();
    let mut stm = db
        .prepare(
            "SELECT * from orders where ordered_at>=?1 and ordered_at<=?2 order by ordered_at ",
        )
        .unwrap();
    let mut orders = stm
        .query_map([start.to_string(), end.to_string()], |r| {
            let mut order = Order::default();
            order.id = Some(r.get_unwrap(0));
            order.card_id = r.get_unwrap(1);
            order.order_count = r.get_unwrap(2);
            order.card_raw_price = r.get_unwrap(3);
            order.card_cost = r.get_unwrap(4);
            order.card_profit = r.get_unwrap(5);
            order.design_cost = r.get_unwrap(6);
            order.discount = r.get_unwrap(7);
            order.customer_paid = r.get_unwrap(8);
            order.ordered_at =
                NaiveDate::parse_from_str(r.get_unwrap::<usize, String>(9).as_str(), "%Y-%m-%d")
                    .ok();
            Ok(order)
        })
        .unwrap();
    //let orders: Vec<Order> = orders.map(|e| e.unwrap()).collect();
    //let table = create_ui_table_from_all_orders(&orders);
    let mut table = Table::new();

    let mut total_profit = 0u32;
    let mut total_initial = 0u32;
    let mut total_paid = 0f64;
    let mut card_portion_of_initial = 0u32;

    for order in orders {
        if let Ok(order) = order {
            total_initial += order.order_count * order.card_cost as u32;
            total_profit +=
                order.order_count * order.card_profit as u32 + order.design_cost - order.discount;
            total_paid += order.customer_paid;
            card_portion_of_initial += order.order_count * order.card_raw_price;
            table.add_row(&order);
        }
    }
    println!("{}", table);
    println!(
        "{:>32} {}",
        "Total Initial:".yellow(),
        format_num!(",.0f", total_initial).green()
    );
    println!(
        "{:>32} {}",
        "Total  Profit:".yellow(),
        format_num!(",.0f", total_profit).green()
    );
    println!(
        "{:>32} {}",
        "Total  Paid:".yellow(),
        format_num!(",.0f", total_paid).blue()
    );
    println!("{}", "-".repeat(44));
    println!(
        "{:>32} {}",
        "Initial        -> Card Portion:".yellow(),
        format_num!(",.0f", card_portion_of_initial).green()
    );
    println!(
        "{:>32} {}\n\n",
        "Initial -> Ink/Printer Portion:".yellow(),
        format_num!(",.0f", total_initial - card_portion_of_initial).green()
    );

    // println!(
    //     "{:>16} {}\n\n",
    //     "Both:".yellow(),
    //     format_num!(",.0f", total_paid).green()
    // );
}
fn add_new_order(
    config: &Config,
    theme: &ColorfulTheme,
    prices: &HashMap<String, Vec<u32>>,
    raw_prices: &HashMap<String, u32>,
) {
    let new_order = Order::new_from_user(config, theme, prices, raw_prices);
    println!("{}", create_ui_table_from_order(&new_order));
    if !Confirm::with_theme(theme)
        .with_prompt("Add this order?")
        .default(true)
        .interact()
        .unwrap()
    {
        return;
    }

    let db = get_db_connection().lock().unwrap();
    match db.execute(
        sql::INSERT_ORDER,
        params![
            new_order.card_id,
            new_order.order_count,
            new_order.card_raw_price,
            new_order.card_cost,
            new_order.card_profit,
            new_order.design_cost,
            new_order.discount,
            new_order.customer_paid,
            new_order.ordered_at.unwrap().to_string(),
        ],
    ) {
        Ok(rows_affected) => {
            println!("{}", "New order added".green());
        }
        Err(e) => {
            println!("{}-{:?}", "Something went wrong adding new record".red(), e);
        }
    }
}
fn manage_orders(
    config: &Config,
    theme: &ColorfulTheme,
    prices: &HashMap<String, Vec<u32>>,
    raw_prices: &HashMap<String, u32>,
) {
    loop {
        match Select::with_theme(theme)
            .items(["Show Orders", "Add New Order", "Back"])
            .default(1)
            .interact()
            .unwrap()
        {
            0 => {
                show_orders(theme);
            }
            1 => {
                add_new_order(config, theme, prices, raw_prices);
            }
            _ => {
                break;
            }
        }
    }
}
fn show_expenses(theme: &ColorfulTheme) {
    match Select::with_theme(theme)
        .with_prompt("Select")
        .items(["Current Month", "Range"])
        .default(0)
        .interact()
        .unwrap()
    {
        0 => {
            let start = parsidate::ParsiDate::today()
                .unwrap()
                .first_day_of_month()
                .to_gregorian()
                .unwrap();
            let end = parsidate::ParsiDate::today()
                .unwrap()
                .last_day_of_month()
                .to_gregorian()
                .unwrap();
            show_expenses_table(start, end);
        }
        1 => {
            let (start, end) = (
                get_parsi_date(theme).to_gregorian().unwrap(),
                get_parsi_date(theme).to_gregorian().unwrap(),
            );
            show_expenses_table(start, end);
        }
        _ => unimplemented!(),
    }
}
fn show_expenses_table(start: NaiveDate, end: NaiveDate) {
    let db = get_db_connection().lock().unwrap();
    let mut stm = db.prepare(SHOW_EXPENSES).unwrap();
    let expenses = stm
        .query_map([start.to_string(), end.to_string()], |row| {
            Ok(Expense {
                id: Some(row.get_unwrap(0)),
                kind: row.get_unwrap(1),
                cost: row.get_unwrap(2),
                description: row.get_unwrap(3),
                issued_at: NaiveDate::parse_from_str(
                    &row.get_unwrap::<usize, String>(4),
                    "%Y-%m-%d",
                )
                .unwrap(),
            })
        })
        .unwrap();
    
    // dbg!(&expenses.collect::<Vec<_>>());
    let mut table = Table::new();
    table.set_header(Row::from(vec!["Id", "Kind", "Cost", "Desc", "Date"]));
    // let mut total_expense = 0u32;
    for expense in expenses {
        if let Ok(expense) = expense {
            // total_expense += expense.cost;
            table.add_row(expense);
        }
    }
    println!("{}", table);
    // println!(
    //     "{} {}\n\n",
    //     "Total Expense :".yellow(),
    //     format_num!(",.0f", total_expense).red()
    // );
}
fn add_new_expense(theme: &ColorfulTheme) {
    let expense = Expense::new_from_user(theme);
    let db = get_db_connection().lock().unwrap();
    db.execute(
        INSERT_EXPENSE,
        params![
            expense.kind,
            expense.cost,
            expense.description,
            expense.issued_at.to_string(),
        ],
    )
    .expect("Cant add new expense");
    println!("{}", "New expense added".green());
}
fn manage_expenses(theme: &ColorfulTheme) {
    loop {
        match Select::with_theme(theme)
            .items(["Show Expenses", "Add New Expense", "Back"])
            .default(1)
            .interact()
            .unwrap()
        {
            0 => {
                show_expenses(theme);
            }
            1 => {
                add_new_expense(theme);
            }
            _ => {
                break;
            }
        }
    }
}

fn show_card_pice(theme: &ColorfulTheme, prices: &HashMap<String, Vec<u32>>) {
    let card_ids = prices.keys().cloned().collect::<Vec<String>>();
    loop {
        let card_id = card_ids[FuzzySelect::with_theme(theme)
            .items(&card_ids)
            .max_length(7)
            .interact()
            .unwrap()]
        .as_str();

        match prices.get(card_id).map(Vec::as_slice) {
            Some(&[lower_price, higher_price, ..]) => {
                let mut table = Table::new();
                table.set_header(["Inventory", "Order"]);
                table.add_row([
                    Cell::from(format_num!(",.0f", lower_price)).fg(Color::Green),
                    Cell::from(format_num!(",.0f", higher_price)).fg(Color::Magenta),
                ]);
                println!("{table}");
            }
            Some(&[lower_price]) => {
                let mut table = Table::new();
                table.set_header(["Inventory"]);
                table.add_row([Cell::from(format_num!(",.0f", lower_price)).fg(Color::Green)]);
                println!("{table}");
            }
            Some(_) => {}
            None => {
                println!("Can not show the result. Maybe there is no row in Price lists!");
            }
        }
        if !Confirm::with_theme(theme)
            .with_prompt("Continue?")
            .default(true)
            .interact()
            .unwrap()
        {
            break;
        }
    }
}
fn get_current_month_tuple() -> (String, ParsiDate, ParsiDate) {
    let today = parsidate::ParsiDate::today().unwrap();
    let start = today.first_day_of_month();
    let end = today.last_day_of_month();
    (MONTHS[today.month() as usize].to_owned(), start, end)
}
fn get_prev_month_tuple() -> (String, ParsiDate, ParsiDate) {
    let today = parsidate::ParsiDate::today().unwrap();
    let today_minus_one_month = today.sub_months(1).unwrap();
    let start = today_minus_one_month.first_day_of_month();
    let end = today_minus_one_month.last_day_of_month();
    (MONTHS[today.month() as usize].to_owned(), start, end)
}
fn manage_report(theme: &ColorfulTheme) {
    let (current_month, cm_start, cm_end) = get_current_month_tuple();
    let (prev_month, pm_start, pm_end) = get_prev_month_tuple();

    // match Select::with_theme(theme).with_prompt("Choose One").items(vec!["From Beginning",""]);
}
fn report(theme: &ColorfulTheme) {
    let db = get_db_connection().lock().unwrap();
    let mut stm = db.prepare(ORDERS_EXPENSE_REPORT).unwrap();
    loop {
        let start = get_parsi_date(theme).to_gregorian().unwrap().to_string();
        let end = get_parsi_date(theme).to_gregorian().unwrap().to_string();
        let mut report: Vec<(u32, u32, u32, u32)> = Vec::new();
        let rows = stm
            .query_map([&start, &end], |row| {
                Ok(InOutReport::new(
                    row.get_unwrap::<usize, String>(0),
                    row.get_unwrap::<usize, u32>(1),
                    row.get_unwrap::<usize, u32>(2),
                    row.get_unwrap::<usize, u32>(3),
                    row.get_unwrap::<usize, u32>(4),
                ))
            })
            .unwrap();
        let mut table = Table::new();
        table.set_header(vec!["", "Card", "Ink/Printer", "Profit", "Balance"]);
        let rows = rows.filter_map(|e| e.ok()).collect::<Vec<_>>();
        let summary = InOutReport::summary(&rows[0], &rows[1]);
        for row in rows {
            table.add_row(row);
        }
        table.add_row(summary);
        println!("\n{}\n", table);
        if !Confirm::with_theme(theme)
            .with_prompt("Continue?")
            .default(true)
            .interact()
            .unwrap()
        {
            break;
        }
    }
}
fn create_auto_price_excel(config: &Config) {
    let tmp = Builder::new()
        .prefix("auto_price_")
        .suffix(".xlsx")
        .tempfile()
        .expect("Can not create Excel temp file");
    let mut workbook = Workbook::new();
    let worksheet = workbook.add_worksheet();
    let auto_prices = get_prices_from_raw_prices(config);
    for (row, (card_id, prices)) in auto_prices.iter().enumerate() {
        worksheet.write(row as u32, 0, card_id);
        worksheet.write(row as u32, 1, prices[0]);
        worksheet.write(row as u32, 2, prices[1]);
    }
    workbook.save(tmp.path());
    let (_, p) = tmp.keep().unwrap();
    open::that(&p);
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    init_db()?;
    let config = read_config("./data.json")?;
    let mut theme = ColorfulTheme {
        active_item_style: Style::new().yellow(),
        ..Default::default()
    };

    let mut raw_prices = read_raw_prices()?;
    let mut prices = get_prices_from_raw_prices(&config);

    loop {
        match Select::with_theme(&theme)
            .with_prompt("Choose")
            .items([
                "Price Calculator",
                "Profit Calculator",
                "Orders",
                "Expenses",
                "Report",
                "Show Card Price",
                "Auto Price Report",
                "Quit",
            ])
            .default(0)
            .interact()
            .unwrap()
        {
            0 => calculate_final_price(&config, &theme),
            1 => calculate_profit2(&config, &theme, &prices),
            2 => manage_orders(&config, &theme, &prices, &raw_prices),
            3 => manage_expenses(&theme),
            4 => report(&theme),
            5 => show_card_pice(&theme, &prices),
            6 => {
                create_auto_price_excel(&config);
            }
            _ => break,
        }
    }
    Ok(())
}
