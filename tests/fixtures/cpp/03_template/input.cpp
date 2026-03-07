#include <iostream>
#include <vector>

template<typename T>
class Stack {
    std::vector<T> data;
public:
    void push(T value) { data.push_back(value); }

    T pop() {
        T val = data.back();
        data.pop_back();
        return val;
    }

    bool empty() const { return data.empty(); }
    size_t size() const { return data.size(); }

    void print() const {
        std::cout << "[";
        for (size_t i = 0; i < data.size(); i++) {
            if (i > 0) std::cout << ", ";
            std::cout << data[i];
        }
        std::cout << "]" << std::endl;
    }
};

int main() {
    Stack<int> s;
    s.push(1);
    s.push(2);
    s.push(3);

    std::cout << "Stack: ";
    s.print();
    std::cout << "Size: " << s.size() << std::endl;

    std::cout << "Pop: " << s.pop() << std::endl;
    std::cout << "Pop: " << s.pop() << std::endl;

    std::cout << "Stack after pops: ";
    s.print();

    return 0;
}
