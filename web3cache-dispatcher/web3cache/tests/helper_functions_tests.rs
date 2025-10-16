use bson::doc;
use web3cache::helper_functions::*;

#[test]
fn test_get_i64_from_doc() {
  let doc = doc! {
      "foo": 42i64,
      "bar": 23i32,
  };

  assert_eq!(get_i64_from_doc(&doc, "foo".to_string()), 42);
  assert_eq!(get_i64_from_doc(&doc, "bar".to_string()), 23);
  assert_eq!(get_i64_from_doc(&doc, "baz".to_string()), 0);
}
