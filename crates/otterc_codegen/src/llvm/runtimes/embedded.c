#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

// Minimal runtime for embedded targets
// No stdio, no system calls - just basic memory operations

int otter_is_valid_utf8(const unsigned char* str, size_t len) {
    size_t i = 0;
    while (i < len) {
        if (str[i] == 0) break;
        int bytes_needed;
        if ((str[i] & 0x80) == 0) {
            bytes_needed = 1;
        } else if ((str[i] & 0xE0) == 0xC0) {
            bytes_needed = 2;
        } else if ((str[i] & 0xF0) == 0xE0) {
            bytes_needed = 3;
        } else if ((str[i] & 0xF8) == 0xF0) {
            bytes_needed = 4;
        } else {
            return 0;
        }
        if (i + bytes_needed > len) return 0;
        for (int j = 1; j < bytes_needed; j++) {
            if ((str[i + j] & 0xC0) != 0x80) return 0;
        }
        i += bytes_needed;
    }
    return 1;
}

char* otter_normalize_text(const char* input) {
    if (!input) return NULL;
    size_t len = strlen(input);
    if (otter_is_valid_utf8((const unsigned char*)input, len)) {
        char* result = (char*)malloc(len + 1);
        if (result) memcpy(result, input, len + 1);
        return result;
    }
    // Simplified: just copy the input
    char* result = (char*)malloc(len + 1);
    if (result) memcpy(result, input, len + 1);
    return result;
}

// Stub implementations for embedded - these would be implemented by the user
void otter_std_io_print(const char* message) {
    (void)message; // Suppress unused warning
    // Implement via UART, SPI, or other hardware interface
}

void otter_std_io_println(const char* message) {
    (void)message;
    // Implement via hardware interface
}

char* otter_std_io_read_line() {
    return NULL; // Not available on embedded
}

void otter_std_io_free_string(char* ptr) {
    if (ptr) free(ptr);
}

int64_t otter_std_time_now_ms() {
    // User must implement hardware timer access
    return 0;
}

char* otter_format_float(double value) {
    (void)value;
    // Minimal implementation - would need custom float formatting
    char* buffer = (char*)malloc(32);
    if (buffer) buffer[0] = '\0';
    return buffer;
}

char* otter_format_int(int64_t value) {
    // Minimal implementation
    char* buffer = (char*)malloc(32);
    if (buffer) buffer[0] = '\0';
    return buffer;
}

char* otter_format_bool(bool value) {
    const char* str = value ? "true" : "false";
    char* buffer = (char*)malloc(strlen(str) + 1);
    if (buffer) memcpy(buffer, str, strlen(str) + 1);
    return buffer;
}

char* otter_str_concat(const char* s1, const char* s2) {
    if (!s1 || !s2) return NULL;
    size_t len1 = strlen(s1), len2 = strlen(s2);
    char* result = (char*)malloc(len1 + len2 + 1);
    if (result) {
        memcpy(result, s1, len1);
        memcpy(result + len1, s2, len2 + 1);
    }
    return result;
}

void otter_free_string(char* ptr) {
    if (ptr) free(ptr);
}

int otter_validate_utf8(const char* ptr) {
    if (!ptr) return 0;
    while (*ptr) {
        unsigned char c = (unsigned char)*ptr;
        if (c <= 0x7F) ptr++;
        else if (c <= 0xDF) {
            if (!ptr[1] || (ptr[1] & 0xC0) != 0x80) return 0;
            ptr += 2;
        } else if (c <= 0xEF) {
            if (!ptr[1] || !ptr[2] || (ptr[1] & 0xC0) != 0x80 || (ptr[2] & 0xC0) != 0x80) return 0;
            ptr += 3;
        } else if (c <= 0xF7) {
            if (!ptr[1] || !ptr[2] || !ptr[3] ||
                (ptr[1] & 0xC0) != 0x80 || (ptr[2] & 0xC0) != 0x80 || (ptr[3] & 0xC0) != 0x80) return 0;
            ptr += 4;
        } else return 0;
    }
    return 1;
}