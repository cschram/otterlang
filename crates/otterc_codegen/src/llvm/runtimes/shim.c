#include <stddef.h>
#include <stdint.h>

extern void otter_entry();

int main(int argc, char** argv) {
    (void)argc;
    (void)argv;
    otter_entry();
    return 0;
}