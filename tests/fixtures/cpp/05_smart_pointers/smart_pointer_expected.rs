use std::sync::Arc;

struct Animal {
    name: String,
    age: i32,
}

impl Animal {
    fn new(name: &str, age: i32) -> Self {
        Self {
            name: name.to_string(),
            age,
        }
    }

    fn speak(&self) {
        println!("{} (age {}) says hello!", self.name, self.age);
    }
}

impl Drop for Animal {
    fn drop(&mut self) {
        println!("{} destroyed", self.name);
    }
}

fn main() {
    // unique_ptr -> Box
    let cat = Box::new(Animal::new("Cat", 3));
    cat.speak();

    // shared_ptr -> Arc
    let dog1 = Arc::new(Animal::new("Dog", 5));
    {
        let dog2 = Arc::clone(&dog1); // shared ownership
        println!("Dog ref count: {}", Arc::strong_count(&dog1));
        dog2.speak();
    }
    println!("Dog ref count after scope: {}", Arc::strong_count(&dog1));

    // Vector of unique_ptr -> Vec<Box<T>>
    let mut zoo: Vec<Box<Animal>> = Vec::new();
    zoo.push(Box::new(Animal::new("Lion", 8)));
    zoo.push(Box::new(Animal::new("Tiger", 6)));

    for animal in &zoo {
        animal.speak();
    }

    println!("End of main");
}
