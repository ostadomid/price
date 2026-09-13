pub fn print_bar_chart(data: &[(&str, u64)], max_value: u64) {
    let bar_width = 40;

    println!("Requests per day\n");

    for (date, value) in data {
        let width = ((*value as f64 / max_value as f64) * bar_width as f64) as usize;
        let bar = "█".repeat(width);

        println!("{:>8} │ {:<40} {:>3}", date, bar, value);
    }

    println!("{:>8} └{}", "", "─".repeat(bar_width + 10));
}