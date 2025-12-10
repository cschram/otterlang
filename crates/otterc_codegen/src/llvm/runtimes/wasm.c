#include <stdint.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>

#ifdef __wasi__
#include <wasi/api.h>
#else
__attribute__((import_module("env"), import_name("otter_write_stdout")))
void otter_env_write_stdout(const char* ptr, uint32_t len);
__attribute__((import_module("env"), import_name("otter_write_stderr")))
void otter_env_write_stderr(const char* ptr, uint32_t len);
__attribute__((import_module("env"), import_name("otter_time_now_ms")))
int64_t otter_env_time_now_ms(void);
#endif

static void otter_write_stdout(const char* data, size_t len) {
    if (!data || len == 0) return;
#ifdef __wasi__
    __wasi_ciovec_t iov = { .buf = data, .buf_len = len };
    size_t written = 0;
    __wasi_fd_write(1, &iov, 1, &written);
#else
    otter_env_write_stdout(data, (uint32_t)len);
#endif
}

static void otter_write_stderr(const char* data, size_t len) {
    if (!data || len == 0) return;
#ifdef __wasi__
    __wasi_ciovec_t iov = { .buf = data, .buf_len = len };
    size_t written = 0;
    __wasi_fd_write(2, &iov, 1, &written);
#else
    otter_env_write_stderr(data, (uint32_t)len);
#endif
}

static char* otter_dup_slice(const char* src, size_t len) {
    if (!src) return NULL;
    char* out = (char*)malloc(len + 1);
    if (!out) return NULL;
    memcpy(out, src, len);
    out[len] = '\0';
    return out;
}

static char* otter_dup_cstr(const char* src) {
    if (!src) return NULL;
    return otter_dup_slice(src, strlen(src));
}

static char* otter_format_signed_uint(uint64_t magnitude, bool negative) {
    char tmp[32];
    size_t idx = 0;
    do {
        tmp[idx++] = (char)('0' + (magnitude % 10));
        magnitude /= 10;
    } while (magnitude > 0);

    size_t total = idx + (negative ? 1 : 0);
    char* buffer = (char*)malloc(total + 1);
    if (!buffer) return NULL;

    size_t pos = 0;
    if (negative) buffer[pos++] = '-';
    while (idx > 0) {
        buffer[pos++] = tmp[--idx];
    }
    buffer[pos] = '\0';
    return buffer;
}

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
    if (!normalized) return;
    otter_write_stdout(normalized, strlen(normalized));
    free(normalized);
}

void otter_std_io_println(const char* message) {
    if (!message) {
        otter_write_stdout("\n", 1);
        return;
    }
    char* normalized = otter_normalize_text(message);
    if (!normalized) return;
    otter_write_stdout(normalized, strlen(normalized));
    otter_write_stdout("\n", 1);
    free(normalized);
}

char* otter_std_io_read_line() {
#ifdef __wasi__
    size_t capacity = 128;
    char* buffer = (char*)malloc(capacity);
    if (!buffer) return NULL;
    size_t len = 0;
    while (1) {
        char ch = 0;
        __wasi_iovec_t iov = { .buf = &ch, .buf_len = 1 };
        size_t nread = 0;
        __wasi_errno_t err = __wasi_fd_read(0, &iov, 1, &nread);
        if (err != __WASI_ERRNO_SUCCESS || nread == 0) break;
        if (ch == '\r') continue;
        if (ch == '\n') break;
        if (len + 1 >= capacity) {
            capacity *= 2;
            char* tmp = (char*)realloc(buffer, capacity);
            if (!tmp) {
                free(buffer);
                return NULL;
            }
            buffer = tmp;
        }
        buffer[len++] = ch;
    }
    if (len == 0) {
        free(buffer);
        return NULL;
    }
    buffer[len] = '\0';
    return buffer;
#else
    return NULL;
#endif
}

void otter_std_io_free_string(char* ptr) {
    if (ptr) free(ptr);
}

int64_t otter_std_time_now_ms() {
#ifdef __wasi__
    __wasi_timestamp_t timestamp = 0;
    __wasi_errno_t err = __wasi_clock_time_get(__WASI_CLOCKID_REALTIME, 1000000, &timestamp);
    if (err != __WASI_ERRNO_SUCCESS) {
        return 0;
    }
    return (int64_t)(timestamp / 1000000);
#else
    return otter_env_time_now_ms();
#endif
}

char* otter_format_int(int64_t value) {
    bool negative = value < 0;
    uint64_t magnitude = negative ? (uint64_t)(-(value + 1)) + 1 : (uint64_t)value;
    return otter_format_signed_uint(magnitude, negative);
}

char* otter_format_float(double value) {
    if (isnan(value)) return otter_dup_cstr("nan");
    if (isinf(value)) return value > 0 ? otter_dup_cstr("inf") : otter_dup_cstr("-inf");

    bool negative = value < 0.0;
    if (negative) value = -value;

    double int_part_d = floor(value);
    if (int_part_d > (double)INT64_MAX) {
        return negative ? otter_dup_cstr("-inf") : otter_dup_cstr("inf");
    }

    int64_t int_part = (int64_t)int_part_d;
    double frac = value - (double)int_part;
    const uint64_t scale = 1000000ULL;
    uint64_t frac_part = (uint64_t)(frac * (double)scale + 0.5);
    if (frac_part >= scale) {
        frac_part -= scale;
        if (int_part < INT64_MAX) {
            int_part += 1;
        }
    }

    char* int_str = otter_format_signed_uint((uint64_t)int_part, negative);
    if (!int_str) return NULL;

    char frac_digits[6] = { '0','0','0','0','0','0' };
    size_t frac_len = 0;
    if (frac_part > 0) {
        for (int i = 5; i >= 0; --i) {
            frac_digits[i] = (char)('0' + (frac_part % 10));
            frac_part /= 10;
        }
        frac_len = 6;
        while (frac_len > 0 && frac_digits[frac_len - 1] == '0') {
            frac_len--;
        }
    }

    if (frac_len == 0) {
        return int_str;
    }

    size_t int_len = strlen(int_str);
    char* result = (char*)malloc(int_len + 1 + frac_len + 1);
    if (!result) {
        free(int_str);
        return NULL;
    }
    memcpy(result, int_str, int_len);
    result[int_len] = '.';
    memcpy(result + int_len + 1, frac_digits, frac_len);
    result[int_len + 1 + frac_len] = '\0';
    free(int_str);
    return result;
}

char* otter_format_bool(bool value) {
    return otter_dup_cstr(value ? "true" : "false");
}

char* otter_str_concat(const char* s1, const char* s2) {
    if (!s1 || !s2) return NULL;
    size_t len1 = strlen(s1), len2 = strlen(s2);
    char* result = (char*)malloc(len1 + len2 + 1);
    if (result) {
        memcpy(result, s1, len1);
        memcpy(result + len1, s2, len2);
        result[len1 + len2] = '\0';
    }
    return result;
}

void otter_free_string(char* ptr) {
    if (ptr) free(ptr);
}

static char* otter_last_error_message = NULL;
static bool otter_has_error_state = false;

bool otter_error_push_context() {
    return true;
}

bool otter_error_pop_context() {
    return true;
}

bool otter_error_raise(const char* message_ptr, size_t message_len) {
    if (otter_last_error_message) {
        free(otter_last_error_message);
        otter_last_error_message = NULL;
    }
    if (message_ptr && message_len > 0) {
        otter_last_error_message = otter_dup_slice(message_ptr, message_len);
    } else {
        const char* fallback = "Exception raised";
        otter_last_error_message = otter_dup_cstr(fallback);
    }
    otter_has_error_state = true;
    if (otter_last_error_message) {
        otter_write_stderr("Exception: ", 11);
        otter_write_stderr(otter_last_error_message, strlen(otter_last_error_message));
        otter_write_stderr("\n", 1);
    }
    return true;
}

bool otter_error_clear() {
    if (otter_last_error_message) {
        free(otter_last_error_message);
        otter_last_error_message = NULL;
    }
    otter_has_error_state = false;
    return true;
}

char* otter_error_get_message() {
    if (!otter_last_error_message) return NULL;
    return otter_dup_cstr(otter_last_error_message);
}

bool otter_error_has_error() {
    return otter_has_error_state;
}

void otter_error_rethrow() {
    // Not implemented for WASM yet
}

char* otter_builtin_stringify_int(int64_t value) {
    return otter_format_int(value);
}

char* otter_builtin_stringify_float(double value) {
    return otter_format_float(value);
}

char* otter_builtin_stringify_bool(int value) {
    return otter_dup_cstr(value ? "true" : "false");
}

void otter_std_fmt_println(const char* msg) {
    otter_std_io_println(msg);
}

void otter_std_fmt_print(const char* msg) {
    otter_std_io_print(msg);
}

void otter_std_fmt_eprintln(const char* msg) {
    if (!msg) {
        otter_write_stderr("\n", 1);
        return;
    }
    char* normalized = otter_normalize_text(msg);
    if (!normalized) return;
    otter_write_stderr(normalized, strlen(normalized));
    otter_write_stderr("\n", 1);
    free(normalized);
}

char* otter_std_fmt_stringify_float(double value) {
    return otter_format_float(value);
}

char* otter_std_fmt_stringify_int(int64_t value) {
    return otter_format_int(value);
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