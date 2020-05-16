use crate::f8::F8;
use num::{Zero, One};

#[test]
fn identities_correct() {
  assert_eq!(F8::zero().v(), 0f32);
  assert_eq!(F8::one().v(), 1f32);
}


#[test]
fn test_from_vals() {
  // assert!(F8::try_from(1.0).is_some());
  let v = F8::approx_from(2.0);
  assert_eq!(v.v(), 2.0);
  /*
  assert!(F8::try_from(0.012).is_some());
  assert!(F8::try_from(0.002).is_some());
  */
}
