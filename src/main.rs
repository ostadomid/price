#![allow(dead_code, unused)]
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
use num_format::{Buffer, Format, Locale::{self, shi}, ToFormattedString};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

static DB_CONNECTION: OnceLock<Mutex<Connection>> = OnceLock::new();

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
    card_raw_price:u32,
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
        raw_prices:&HashMap<String, u32>,
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
            .interact_text()
            .unwrap();
        let design_cost = Input::<u32>::with_theme(theme)
            .with_prompt("Design Cost")
            .default(if order_count < 200 {
                400_000
            } else {
                order_count * 2000
            })
            .interact_text()
            .unwrap_or_default();
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
            self.id.unwrap_or_default().to_string(),
            self.card_id.to_string(),
            self.order_count.to_string(),
            self.card_raw_price.to_string(),
            self.card_cost.to_string(),
            self.card_profit.to_string(),
            self.design_cost.to_string(),
            self.discount.to_string(),
            self.customer_paid.to_string(),
            self.ordered_at.unwrap_or_default().to_string(),
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
        'inner: loop {
            if cost_shipment == 0 {
                return;
            }
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
                
            let shipment_price = if let Some(sh) = shipment{
                sh.value()
            } else{
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

            (card_code.clone(), vec![lower_price, higher_price])
        })
        .collect::<HashMap<String, Vec<u32>>>();
    result
}
fn init_db() -> Result<(), Box<dyn Error>> {
    let c = Connection::open("./databse.sqlite").expect("Cant find/create database.sqlite");
    c.execute(sql::CREATE_ORDER_TABLE, [])
        .expect("Cant create table");
    let _ = DB_CONNECTION.set(Mutex::new(c));
    Ok(())
}
fn get_db_connection() -> &'static Mutex<Connection> {
    DB_CONNECTION.get().unwrap()
}
fn create_ui_table_from_all_orders(orders: &Vec<Order>) -> Table {
    let mut table = Table::new();
    table.set_header(Row::from([
        "Id", "Code", "Count", "Raw Price","Cost", "Profit", "Design", "Discount", "Paid", "Date",
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
            for day in 1..31 {
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
    let db = get_db_connection().lock().unwrap();
    let mut stm = db.prepare("SELECT * from orders").unwrap();
    let mut rows = stm
        .query_map([], |r| {
            let mut order = Order::default();
            order.id = Some(r.get_unwrap(0));
            order.card_id = r.get_unwrap(1);
            order.order_count = r.get_unwrap(2);
            order.card_cost = r.get_unwrap(3);
            order.card_profit = r.get_unwrap(4);
            order.design_cost = r.get_unwrap(5);
            order.discount = r.get_unwrap(6);
            order.customer_paid = r.get_unwrap(7);
            order.ordered_at =
                NaiveDate::parse_from_str(r.get_unwrap::<usize, String>(8).as_str(), "").ok();
            Ok(order)
        })
        .unwrap();
    let orders: Vec<Order> = rows.map(|e| e.unwrap()).collect();
    let table = create_ui_table_from_all_orders(&orders);
    println!("{table}");
}
fn add_new_order(config: &Config, theme: &ColorfulTheme, prices: &HashMap<String, Vec<u32>>, raw_prices: &HashMap<String,u32>) {
    let new_order = Order::new_from_user(config, theme, prices,raw_prices);
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
fn manage_orders(config: &Config, theme: &ColorfulTheme, prices: &HashMap<String, Vec<u32>>, raw_prices: &HashMap<String,u32>) {
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
                add_new_order(config, theme, prices,raw_prices);
            }
            _ => {
                break;
            }
        }
    }
}
fn compare_prices(
    theme: &ColorfulTheme,
    prices: &HashMap<String, Vec<u32>>,
    auto_prices: &HashMap<String, Vec<u32>>,
) {
    let card_ids = prices.keys().cloned().collect::<Vec<String>>();
    let card_id = card_ids[FuzzySelect::with_theme(theme)
        .items(&card_ids)
        .max_length(7)
        .interact()
        .unwrap()].as_str();
    println!(
        "Manual: {:?}  Auto: {:?}",
        prices.get(card_id).unwrap(),
        auto_prices.get(card_id).unwrap()
    );
}
#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    init_db()?;
    let config = read_config("./data.json")?;
    let mut theme = ColorfulTheme {
        active_item_style: Style::new().yellow(),
        ..Default::default()
    };
    let mut prices = read_prices()?;
    let mut raw_prices = read_raw_prices()?;
    let mut auto_prices = get_prices_from_raw_prices(&config);

    loop {
        match Select::with_theme(&theme)
            .with_prompt("Choose")
            .items(["Final Price", "Profit", "Orders", "Compare Prices", "Quit"])
            .default(0)
            .interact()
            .unwrap()
        {
            0 => calculate_final_price(&config, &theme),
            1 => calculate_profit2(&config, &theme, &prices),
            2 => manage_orders(&config, &theme, &prices,&raw_prices),
            3 => compare_prices(&theme, &prices, &auto_prices),
            _ => break,
        }
    }
    Ok(())
}
