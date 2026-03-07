use sqlite3_utf::{utf8_char_len, utf8_read, utf16le_to_string, utf8_to_utf16le};

fn main() {
    // Demonstrate equivalence with sqlite3's UTF functions
    let text = "Hello, SQLite! こんにちは 😀";
    let bytes = text.as_bytes();

    println!("Input: {}", text);
    println!("Byte length: {}", bytes.len());
    println!(
        "Char length (sqlite3Utf8CharLen): {}",
        utf8_char_len(bytes, Some(bytes.len()))
    );

    // Read first 5 codepoints
    let mut pos = 0;
    print!("First 5 codepoints: ");
    for _ in 0..5 {
        let ch = utf8_read(bytes, &mut pos);
        print!("U+{:04X}({}) ", ch as u32, ch);
    }
    println!();

    // UTF-16 round trip
    let utf16 = utf8_to_utf16le("SQLite");
    let back = utf16le_to_string(&utf16);
    println!("UTF-16 LE roundtrip: {} -> {} bytes -> {}", "SQLite", utf16.len(), back);
}
