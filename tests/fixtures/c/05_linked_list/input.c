#include <stdio.h>
#include <stdlib.h>

typedef struct Node {
    int data;
    struct Node* next;
} Node;

Node* create_node(int data) {
    Node* node = (Node*)malloc(sizeof(Node));
    if (!node) return NULL;
    node->data = data;
    node->next = NULL;
    return node;
}

void push_front(Node** head, int data) {
    Node* node = create_node(data);
    if (!node) return;
    node->next = *head;
    *head = node;
}

void print_list(const Node* head) {
    const Node* current = head;
    while (current) {
        printf("%d", current->data);
        if (current->next) printf(" -> ");
        current = current->next;
    }
    printf("\n");
}

int list_length(const Node* head) {
    int count = 0;
    const Node* current = head;
    while (current) {
        count++;
        current = current->next;
    }
    return count;
}

void free_list(Node* head) {
    while (head) {
        Node* temp = head;
        head = head->next;
        free(temp);
    }
}

int main() {
    Node* list = NULL;

    push_front(&list, 3);
    push_front(&list, 2);
    push_front(&list, 1);

    printf("List: ");
    print_list(list);
    printf("Length: %d\n", list_length(list));

    free_list(list);
    return 0;
}
