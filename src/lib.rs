#[cfg(test)]
pub mod tests{
  #[test]
  pub fn one(){
    let today = parsidate::ParsiDate::today().unwrap();
    let start = today.first_day_of_month();
    let end = today.last_day_of_month();
    println!("{}-{}",start,end);
    let today =today.sub_months(1).unwrap();
    let start = today.first_day_of_month();
    let end = today.last_day_of_month();
    println!("{}-{}",start,end);


    // assert_eq!(last_month.month(), 1);
    // assert_eq!(last_month.day(), 31);
  }
}