/*
 * VEGA Pilot Firmware - Minimal libc string/memory implementations
 * For -ffreestanding -nostdlib builds
 */

#include <stdint.h>
#include <stddef.h>
#include <stdarg.h>

/* errno stub for libm (freestanding has no libc errno) */
int __errno;

void *memcpy(void *dest, const void *src, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    const uint8_t *s = (const uint8_t *)src;
    while (n--) {
        *d++ = *s++;
    }
    return dest;
}

void *memset(void *s, int c, size_t n) {
    uint8_t *p = (uint8_t *)s;
    while (n--) {
        *p++ = (uint8_t)c;
    }
    return s;
}

int memcmp(const void *s1, const void *s2, size_t n) {
    const uint8_t *p1 = (const uint8_t *)s1;
    const uint8_t *p2 = (const uint8_t *)s2;
    while (n--) {
        if (*p1 != *p2) {
            return *p1 - *p2;
        }
        p1++;
        p2++;
    }
    return 0;
}

void *memmove(void *dest, const void *src, size_t n) {
    uint8_t *d = (uint8_t *)dest;
    const uint8_t *s = (const uint8_t *)src;
    if (d < s) {
        while (n--) {
            *d++ = *s++;
        }
    } else {
        d += n;
        s += n;
        while (n--) {
            *--d = *--s;
        }
    }
    return dest;
}

size_t strlen(const char *s) {
    size_t len = 0;
    while (*s++) len++;
    return len;
}

char *strcpy(char *dest, const char *src) {
    char *d = dest;
    while ((*d++ = *src++));
    return dest;
}

char *strncpy(char *dest, const char *src, size_t n) {
    char *d = dest;
    while (n-- && (*d++ = *src++));
    while (n--) *d++ = '\0';
    return dest;
}

int strcmp(const char *s1, const char *s2) {
    while (*s1 && (*s1 == *s2)) {
        s1++;
        s2++;
    }
    return *(const unsigned char *)s1 - *(const unsigned char *)s2;
}

int strncmp(const char *s1, const char *s2, size_t n) {
    while (n-- && *s1 && (*s1 == *s2)) {
        s1++;
        s2++;
    }
    if (n == (size_t)-1) return 0;
    return *(const unsigned char *)s1 - *(const unsigned char *)s2;
}

char *strcat(char *dest, const char *src) {
    char *d = dest + strlen(dest);
    while ((*d++ = *src++));
    return dest;
}

char *strchr(const char *s, int c) {
    while (*s) {
        if (*s == (char)c) return (char *)s;
        s++;
    }
    return NULL;
}

void *memchr(const void *s, int c, size_t n) {
    const uint8_t *p = (const uint8_t *)s;
    while (n--) {
        if (*p == (uint8_t)c) return (void *)p;
        p++;
    }
    return NULL;
}

/* Minimal snprintf implementation */
int vsnprintf(char *str, size_t size, const char *format, va_list args);

int snprintf(char *str, size_t size, const char *format, ...) {
    va_list args;
    va_start(args, format);
    int ret = vsnprintf(str, size, format, args);
    va_end(args);
    return ret;
}

/* Minimal vsnprintf implementation - supports %d, %u, %x, %s, %c,
 * plus 'l' length modifier (%lu, %ld, %lx). On RV32ILP32 long is 32-bit,
 * so %lu==%u, %ld==%d, %lx==%x. */
int vsnprintf(char *str, size_t size, const char *format, va_list args) {
    char *out = str;
    size_t remaining = size;
    
    if (size == 0) return 0;
    
    while (*format && remaining > 1) {
        if (*format == '%') {
            format++;
            if (*format == 'l') {
                format++;  /* consume 'l': long is 32-bit here, same as int */
            }
            if (*format == 'd') {
                int val = va_arg(args, int);
                char buf[16];
                if (val < 0) {
                    if (remaining > 1) { *out++ = '-'; remaining--; }
                    val = -val;
                }
                // Convert to decimal
                char *p = buf + 15;
                *p = '\0';
                if (val == 0) *--p = '0';
                while (val > 0) {
                    *--p = '0' + (val % 10);
                    val /= 10;
                }
                while (*p) {
                    if (remaining <= 1) break;
                    *out++ = *p++;
                    remaining--;
                }
            } else if (*format == 'u') {
                unsigned int val = va_arg(args, unsigned int);
                char buf[16];
                char *p = buf + 15;
                *p = '\0';
                if (val == 0) *--p = '0';
                while (val > 0) {
                    *--p = '0' + (val % 10);
                    val /= 10;
                }
                while (*p) {
                    if (remaining <= 1) break;
                    *out++ = *p++;
                    remaining--;
                }
            } else if (*format == 'x') {
                unsigned int val = va_arg(args, unsigned int);
                char buf[16];
                char *p = buf + 15;
                *p = '\0';
                if (val == 0) *--p = '0';
                while (val > 0) {
                    int digit = val % 16;
                    *--p = (digit < 10) ? '0' + digit : 'a' + digit - 10;
                    val /= 16;
                }
                while (*p) {
                    if (remaining <= 1) break;
                    *out++ = *p++;
                    remaining--;
                }
            } else if (*format == 's') {
                const char *s = va_arg(args, const char*);
                while (*s && remaining > 1) {
                    *out++ = *s++;
                    remaining--;
                }
            } else if (*format == 'c') {
                char c = (char)va_arg(args, int);
                if (remaining > 1) {
                    *out++ = c;
                    remaining--;
                }
            } else {
                if (remaining > 1) {
                    *out++ = '%';
                    remaining--;
                }
                if (remaining > 1) {
                    *out++ = *format;
                    remaining--;
                }
            }
        } else {
            if (remaining > 1) {
                *out++ = *format;
                remaining--;
            }
        }
        format++;
    }
    *out = '\0';
    return size - remaining;
}