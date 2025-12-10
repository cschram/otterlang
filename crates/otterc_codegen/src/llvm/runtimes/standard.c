#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <stdbool.h>
#include <ctype.h>
#ifndef _WIN32
#include <sys/time.h>
#include <sys/types.h>
#else
#define WIN32_LEAN_AND_MEAN
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <BaseTsd.h>
typedef SSIZE_T ssize_t;

struct timeval {
    long tv_sec;
    long tv_usec;
};

static int gettimeofday(struct timeval* tv, void* tz) {
    (void)tz;
    if (!tv) {
        return -1;
    }
    FILETIME ft;
    ULONGLONG timestamp;
    static const ULONGLONG EPOCH_OFFSET = 116444736000000000ULL;
    GetSystemTimeAsFileTime(&ft);
    timestamp = ((ULONGLONG)ft.dwHighDateTime << 32) | ft.dwLowDateTime;
    timestamp -= EPOCH_OFFSET;
    tv->tv_sec = (long)(timestamp / 10000000ULL);
    tv->tv_usec = (long)((timestamp % 10000000ULL) / 10ULL);
    return 0;
}

static ssize_t otter_getline(char** lineptr, size_t* n, FILE* stream) {
    if (!lineptr || !n || !stream) {
        return -1;
    }
    if (*lineptr == NULL || *n == 0) {
        *n = 128;
        *lineptr = (char*)malloc(*n);
        if (!*lineptr) {
            return -1;
        }
    }

    size_t position = 0;
    for (;;) {
        int c = fgetc(stream);
        if (c == EOF) {
            if (position == 0) {
                return -1;
            }
            break;
        }
        if (position + 1 >= *n) {
            size_t new_size = *n * 2;
            char* new_ptr = (char*)realloc(*lineptr, new_size);
            if (!new_ptr) {
                return -1;
            }
            *lineptr = new_ptr;
            *n = new_size;
        }
        (*lineptr)[position++] = (char)c;
        if (c == '\n') {
            break;
        }
    }
    (*lineptr)[position] = '\0';
    return (ssize_t)position;
}

#define getline otter_getline
#endif

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
        if (result) {
            memcpy(result, input, len + 1);
        }
        return result;
    }
    char* result = (char*)malloc(len * 3 + 1);
    if (!result) return NULL;
    size_t i = 0, out_pos = 0;
    while (i < len) {
        unsigned char c = (unsigned char)input[i];
        if (c == 0) break;
        int bytes_needed = 0, valid_sequence = 1;
        if ((c & 0x80) == 0) bytes_needed = 1;
        else if ((c & 0xE0) == 0xC0) bytes_needed = 2;
        else if ((c & 0xF0) == 0xE0) bytes_needed = 3;
        else if ((c & 0xF8) == 0xF0) bytes_needed = 4;
        else { valid_sequence = 0; bytes_needed = 1; }
        if (i + bytes_needed > len) valid_sequence = 0;
        else if (bytes_needed > 1) {
            for (int j = 1; j < bytes_needed && valid_sequence; j++) {
                if ((input[i + j] & 0xC0) != 0x80) valid_sequence = 0;
            }
        }
        if (valid_sequence) {
            for (int j = 0; j < bytes_needed; j++) result[out_pos++] = input[i + j];
            i += bytes_needed;
        } else {
            result[out_pos++] = (char)0xEF;
            result[out_pos++] = (char)0xBF;
            result[out_pos++] = (char)0xBD;
            i++;
        }
    }
    result[out_pos] = '\0';
    return result;
}

void otter_std_io_print(const char* message) {
    if (!message) return;
    char* normalized = otter_normalize_text(message);
    if (normalized) {
        printf("%s", normalized);
        fflush(stdout);
        free(normalized);
    }
}

void otter_std_io_println(const char* message) {
    if (!message) {
        printf("\n");
        return;
    }
    char* normalized = otter_normalize_text(message);
    if (normalized) {
        printf("%s\n", normalized);
        free(normalized);
    }
}

char* otter_std_io_read_line() {
    char* line = NULL;
    size_t len = 0;
    ssize_t read = getline(&line, &len, stdin);
    if (read == -1) {
        free(line);
        return NULL;
    }
    if (read > 0 && line[read-1] == '\n') {
        line[read-1] = '\0';
    }
    return line;
}

void otter_std_io_free_string(char* ptr) {
    if (ptr) free(ptr);
}

int64_t otter_std_time_now_ms() {
    struct timeval tv;
    gettimeofday(&tv, NULL);
    return (int64_t)tv.tv_sec * 1000 + tv.tv_usec / 1000;
}

char* otter_format_float(double value) {
    char* buffer = (char*)malloc(64);
    if (buffer) {
        int len = snprintf(buffer, 64, "%.9f", value);
        if (len > 0) {
            char* p = buffer + len - 1;
            while (p > buffer && *p == '0') {
                *p = '\0';
                p--;
            }
            if (p > buffer && *p == '.') *p = '\0';
        }
    }
    return buffer;
}

char* otter_format_int(int64_t value) {
    char* buffer = (char*)malloc(32);
    if (buffer) snprintf(buffer, 32, "%lld", (long long)value);
    return buffer;
}

char* otter_format_bool(bool value) {
    const char* str = value ? "true" : "false";
    size_t len = strlen(str);
    char* buffer = (char*)malloc(len + 1);
    if (buffer) {
        memcpy(buffer, str, len + 1);
    }
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


// Exception handling with flag-based approach
typedef struct ExceptionContext {
    char* error_message;
    size_t error_message_len;
    bool has_error;
    struct ExceptionContext* prev;
} ExceptionContext;

static __thread ExceptionContext* context_stack = NULL;

void otter_error_push_context() {
    ExceptionContext* ctx = (ExceptionContext*)malloc(sizeof(ExceptionContext));
    if (!ctx) return;
    
    ctx->error_message = NULL;
    ctx->error_message_len = 0;
    ctx->has_error = false;
    ctx->prev = context_stack;
    context_stack = ctx;
}

bool otter_error_pop_context() {
    if (!context_stack) return false;
    
    ExceptionContext* ctx = context_stack;
    context_stack = ctx->prev;
    
    if (ctx->error_message) {
        free(ctx->error_message);
    }
    free(ctx);
    
    return true;
}

void otter_error_raise(const char* message_ptr, size_t message_len) {
    if (!context_stack) {
        // No exception handler - print and abort
        if (message_ptr && message_len > 0) {
            fprintf(stderr, "Uncaught exception: %.*s\n", (int)message_len, message_ptr);
        } else {
            fprintf(stderr, "Uncaught exception\n");
        }
        abort();
    }
    
    // Store error message in current context
    context_stack->has_error = true;
    context_stack->error_message_len = message_len;
    
    if (message_ptr && message_len > 0) {
        context_stack->error_message = (char*)malloc(message_len + 1);
        if (context_stack->error_message) {
            memcpy(context_stack->error_message, message_ptr, message_len);
            context_stack->error_message[message_len] = '\0';
        }
    }
}

bool otter_error_clear() {
    if (!context_stack) return false;
    
    context_stack->has_error = false;
    if (context_stack->error_message) {
        free(context_stack->error_message);
        context_stack->error_message = NULL;
    }
    context_stack->error_message_len = 0;
    
    return true;
}

char* otter_error_get_message() {
    if (!context_stack || !context_stack->error_message) {
        char* result = (char*)malloc(1);
        if (result) result[0] = '\0';
        return result;
    }
    
    // Return a copy of the error message
    char* result = (char*)malloc(context_stack->error_message_len + 1);
    if (result) {
        memcpy(result, context_stack->error_message, context_stack->error_message_len);
        result[context_stack->error_message_len] = '\0';
    }
    return result;
}

bool otter_error_has_error() {
    return context_stack && context_stack->has_error;
}

void otter_error_rethrow() {
    if (!context_stack || !context_stack->has_error) return;
    
    // If there's a previous context, copy error to it
    if (context_stack->prev) {
        ExceptionContext* prev = context_stack->prev;
        prev->has_error = true;
        prev->error_message_len = context_stack->error_message_len;
        
        if (context_stack->error_message) {
            prev->error_message = (char*)malloc(context_stack->error_message_len + 1);
            if (prev->error_message) {
                memcpy(prev->error_message, context_stack->error_message, context_stack->error_message_len);
                prev->error_message[context_stack->error_message_len] = '\0';
            }
        }
    }
    // Just return - the unreachable after rethrow will prevent further execution
}

// Personality function for LLVM exception handling
// This is called by LLVM during exception unwinding
int otter_personality(int version, int actions, uint64_t exception_class,
                      void* exception_object, void* context) {
    // For our simple exception model, we always claim we can handle the exception
    // Return 0 (_URC_NO_REASON) to indicate successful handling
    return 0;
}

char* otter_builtin_stringify_int(int64_t value) {
    char* buffer = (char*)malloc(32);
    if (buffer) {
        snprintf(buffer, 32, "%lld", (long long)value);
    }
    return buffer;
}

char* otter_builtin_stringify_float(double value) {
    char* buffer = (char*)malloc(64);
    if (buffer) {
        int len = snprintf(buffer, 64, "%.9f", value);
        if (len > 0) {
            char* p = buffer + len - 1;
            while (p > buffer && *p == '0') {
                *p = '0';
                p--;
            }
            if (p > buffer && *p == '.') *p = '0';
        }
    }
    return buffer;
}

char* otter_builtin_stringify_bool(int value) {
    char* buffer = (char*)malloc(6);
    if (buffer) {
        const char* str = value ? "true" : "false";
        size_t len = value ? 4 : 5;
        memcpy(buffer, str, len + 1);
    }
    return buffer;
}


void otter_std_fmt_println(const char* msg) {
    if (!msg) {
        printf("n");
        return;
    }
    char* normalized = otter_normalize_text(msg);
    if (normalized) {
        printf("%sn", normalized);
        free(normalized);
    }
}

void otter_std_fmt_print(const char* msg) {
    if (!msg) return;
    char* normalized = otter_normalize_text(msg);
    if (normalized) {
        printf("%s", normalized);
        fflush(stdout);
        free(normalized);
    }
}

void otter_std_fmt_eprintln(const char* msg) {
    if (!msg) {
        fprintf(stderr, "n");
        return;
    }
    char* normalized = otter_normalize_text(msg);
    if (normalized) {
        fprintf(stderr, "%sn", normalized);
        free(normalized);
    }
}

char* otter_std_fmt_stringify_float(double value) {
    char* buffer = (char*)malloc(64);
    if (buffer) {
        int len = snprintf(buffer, 64, "%.9f", value);
        if (len > 0) {
            char* p = buffer + len - 1;
            while (p > buffer && *p == '0') {
                *p = '0';
                p--;
            }
            if (p > buffer && *p == '.') *p = '0';
        }
    }
    return buffer;
}

char* otter_std_fmt_stringify_int(int64_t value) {
    char* buffer = (char*)malloc(32);
    if (buffer) {
        snprintf(buffer, 32, "%lld", (long long)value);
    }
    return buffer;
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

int64_t otter_builtin_len_string(const char* s) {
    if (!s) return 0;
    return (int64_t)strlen(s);
}

extern void otter_entry();
int main(int argc, char** argv) {
    otter_entry();
    return 0;
}