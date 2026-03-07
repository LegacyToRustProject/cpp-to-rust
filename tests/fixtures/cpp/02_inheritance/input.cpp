#include <iostream>
#include <string>
#include <cmath>

class Shape {
public:
    virtual double area() const = 0;
    virtual std::string name() const = 0;
    virtual ~Shape() = default;

    void print() const {
        std::cout << name() << ": area = " << area() << std::endl;
    }
};

class Circle : public Shape {
    double radius;
public:
    Circle(double r) : radius(r) {}
    double area() const override { return M_PI * radius * radius; }
    std::string name() const override { return "Circle"; }
};

class Square : public Shape {
    double side;
public:
    Square(double s) : side(s) {}
    double area() const override { return side * side; }
    std::string name() const override { return "Square"; }
};

int main() {
    Circle c(5.0);
    Square s(4.0);

    c.print();
    s.print();

    return 0;
}
