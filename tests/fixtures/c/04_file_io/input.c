#include <stdio.h>
#include <string.h>

int main() {
    // Write to file
    FILE* f = fopen("/tmp/cpp_to_rust_test.txt", "w");
    if (!f) {
        fprintf(stderr, "Cannot open file for writing\n");
        return 1;
    }
    fprintf(f, "Line 1: Hello\n");
    fprintf(f, "Line 2: World\n");
    fprintf(f, "Line 3: Test\n");
    fclose(f);

    // Read from file
    f = fopen("/tmp/cpp_to_rust_test.txt", "r");
    if (!f) {
        fprintf(stderr, "Cannot open file for reading\n");
        return 1;
    }

    char line[256];
    int count = 0;
    while (fgets(line, sizeof(line), f)) {
        // Remove newline
        line[strcspn(line, "\n")] = 0;
        printf("Read: %s\n", line);
        count++;
    }
    fclose(f);

    printf("Total lines: %d\n", count);
    remove("/tmp/cpp_to_rust_test.txt");
    return 0;
}
