#ifndef LION_C_API_H
#define LION_C_API_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef _MSC_VER
#define LION_EXPORT __declspec(dllexport)
#elif defined(__GNUC__) || defined(__clang__)
#define LION_EXPORT __attribute__((visibility("default")))
#else
#define LION_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    LION_NIL = 0,
    LION_INT = 1,
    LION_FLOAT = 2,
    LION_BOOL = 3,
    LION_STRING = 4,
} LionType;

typedef struct {
    const uint8_t* ptr;
    size_t len;
} StrData;

typedef union {
    int64_t as_int;
    double as_float;
    uint8_t as_bool;
    StrData as_str;
} LionValueData;

typedef struct {
    int32_t tag;
    LionValueData data;
} LionValue;

typedef struct {
    const char* name;
    LionValue (*func)(int32_t argc, const LionValue* args);
} LionModuleFunc;

LION_EXPORT int32_t lion_module_init(int32_t* out_count, LionModuleFunc** out_funcs);

#ifdef __cplusplus
}
#endif

#endif // LION_C_API_H
