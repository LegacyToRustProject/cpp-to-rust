struct Person {
    name: String,
    age: i32,
    height: f64,
}

fn create_person(name: &str, age: i32, height: f64) -> Person {
    Person {
        name: name.to_string(),
        age,
        height,
    }
}

fn print_person(p: &Person) {
    println!("Name: {}, Age: {}, Height: {:.1}", p.name, p.age, p.height);
}

fn main() {
    let alice = create_person("Alice", 30, 165.5);
    let bob = create_person("Bob", 25, 180.0);

    print_person(&alice);
    print_person(&bob);

    // No free() needed - Rust's ownership system handles deallocation (RAII)
}
