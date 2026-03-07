#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
    char name[64];
    int age;
    double height;
} Person;

Person* create_person(const char* name, int age, double height) {
    Person* p = (Person*)malloc(sizeof(Person));
    if (!p) return NULL;
    strncpy(p->name, name, 63);
    p->name[63] = '\0';
    p->age = age;
    p->height = height;
    return p;
}

void print_person(const Person* p) {
    printf("Name: %s, Age: %d, Height: %.1f\n", p->name, p->age, p->height);
}

int main() {
    Person* alice = create_person("Alice", 30, 165.5);
    Person* bob = create_person("Bob", 25, 180.0);

    if (alice) print_person(alice);
    if (bob) print_person(bob);

    free(alice);
    free(bob);
    return 0;
}
