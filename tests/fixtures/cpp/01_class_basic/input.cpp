#include <iostream>
#include <string>

class Rectangle {
private:
    double width;
    double height;

public:
    Rectangle(double w, double h) : width(w), height(h) {}

    double area() const {
        return width * height;
    }

    double perimeter() const {
        return 2 * (width + height);
    }

    std::string to_string() const {
        return "Rectangle(" + std::to_string(width) + " x " + std::to_string(height) + ")";
    }
};

int main() {
    Rectangle r(5.0, 3.0);
    std::cout << r.to_string() << std::endl;
    std::cout << "Area: " << r.area() << std::endl;
    std::cout << "Perimeter: " << r.perimeter() << std::endl;
    return 0;
}
