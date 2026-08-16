#[cfg(test)]
pub mod tests{
    use std::{cell::{OnceCell, RefCell}, rc::Rc};

  #[test]
  pub fn one(){
    struct Student{
      name:String,
      age:u8
    }
    let a = Rc::new(RefCell::new(Student{name:"bob".into(),age:5}));
    a.borrow_mut().age+=1;
    let a = OnceCell::new();
    let r = a.get_or_init(||12);
  }
}