#![allow(dead_code, unused)]

#[cfg(test)]
pub mod tests {
    use std::error::Error;

    #[test]
    pub fn one() -> Result<(), Box<dyn Error>> {
        let year = parsidate::ParsiDate::today()?.year();
        let mut dates = Vec::<String>::with_capacity(366 * 2);
        for y in year..=year + 1 {
            for m in 1..=12 {
                for d in 1..=31 {
                    let instance = unsafe { parsidate::ParsiDate::new_unchecked(y, m, d) };
                    if instance.is_valid() {
                        dates.push(instance.format("short"));
                    }
                }
            }
        }
        type DateRange = (String, String);

        let today = parsidate::ParsiDate::today()?;
        let today_in_last_month = today.sub_months(1)?;
        today.to_gregorian()?.format("%Y-%m-%d");
        let current_month: DateRange = (
            today.first_day_of_month().format("short"),
            today.last_day_of_month().format("short"),
        );
        let prev_month: DateRange = (
            today_in_last_month.first_day_of_month().format("short"),
            today_in_last_month.last_day_of_month().format("short"),
        );
    

        Ok(())
    }
    #[test]
    pub fn two(){
        let mut b = std::collections::BTreeMap::new();
        b.insert("M-339", 4200);
        b.insert("AL-333", 8500);
        b.insert("H-1400", 5500);
        let x = b.entry("al").or_default();
        for k in b.keys(){
            println!("{}",k);
        }
    }
}
