#include <iostream>
#include <vector>
#include <map>
#include <algorithm>

int main() {
    // Vector operations
    std::vector<int> nums = {5, 2, 8, 1, 9, 3};

    std::sort(nums.begin(), nums.end());

    std::cout << "Sorted: ";
    for (int n : nums) {
        std::cout << n << " ";
    }
    std::cout << std::endl;

    // Map operations
    std::map<std::string, int> ages;
    ages["Alice"] = 30;
    ages["Bob"] = 25;
    ages["Charlie"] = 35;

    for (const auto& [name, age] : ages) {
        std::cout << name << ": " << age << std::endl;
    }

    // Find
    auto it = ages.find("Bob");
    if (it != ages.end()) {
        std::cout << "Found Bob, age: " << it->second << std::endl;
    }

    return 0;
}
