// Test program equivalent to the sprint's test_cjson.c:
//   gcc /tmp/test_cjson.c cJSON.c -o test-cjson-orig
//   ./test-cjson-orig
// Expected output:
//   name: John
//   age: 30

use cjson_rs::JsonValue;

fn main() {
    let json = r#"{"name":"John","age":30}"#;
    let root = JsonValue::parse(json).expect("parse failed");

    let name = root.get_object_item("name").expect("name not found");
    let age = root.get_object_item("age").expect("age not found");

    println!("name: {}", name.as_str().unwrap());
    println!("age: {}", age.as_i64().unwrap());

    // cJSON_Delete(root) — no-op in Rust, drop handles it
    root.delete();
}
