#include <iostream>
#include <memory>
#include <string>
#include <vector>

class Animal {
    std::string name;
    int age;
public:
    Animal(const std::string& n, int a) : name(n), age(a) {}
    void speak() const {
        std::cout << name << " (age " << age << ") says hello!" << std::endl;
    }
    ~Animal() {
        std::cout << name << " destroyed" << std::endl;
    }
};

int main() {
    // unique_ptr
    auto cat = std::make_unique<Animal>("Cat", 3);
    cat->speak();

    // shared_ptr
    auto dog1 = std::make_shared<Animal>("Dog", 5);
    {
        auto dog2 = dog1; // shared ownership
        std::cout << "Dog ref count: " << dog1.use_count() << std::endl;
        dog2->speak();
    }
    std::cout << "Dog ref count after scope: " << dog1.use_count() << std::endl;

    // Vector of unique_ptr
    std::vector<std::unique_ptr<Animal>> zoo;
    zoo.push_back(std::make_unique<Animal>("Lion", 8));
    zoo.push_back(std::make_unique<Animal>("Tiger", 6));

    for (const auto& animal : zoo) {
        animal->speak();
    }

    std::cout << "End of main" << std::endl;
    return 0;
}
