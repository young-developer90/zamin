#ifndef ZAMIN_C_API_H
#define ZAMIN_C_API_H

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

#ifdef _MSC_VER
#define ZAMIN_EXPORT __declspec(dllexport)
#elif defined(__GNUC__) || defined(__clang__)
#define ZAMIN_EXPORT __attribute__((visibility("default")))
#else
#define ZAMIN_EXPORT
#endif

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    ZAMIN_NIL = 0,
    ZAMIN_INT = 1,
    ZAMIN_FLOAT = 2,
    ZAMIN_BOOL = 3,
    ZAMIN_STRING = 4,
} ZaminType;

typedef struct {
    const uint8_t* ptr;
    size_t len;
} StrData;

typedef union {
    int64_t as_int;
    double as_float;
    uint8_t as_bool;
    StrData as_str;
} ZaminValueData;

typedef struct {
    int32_t tag;
    ZaminValueData data;
} ZaminValue;

typedef struct {
    const char* name;
    ZaminValue (*func)(int32_t argc, const ZaminValue* args);
} ZaminModuleFunc;

ZAMIN_EXPORT int32_t zamin_module_init(int32_t* out_count, ZaminModuleFunc** out_funcs);

#ifdef __cplusplus
}
#endif

#endif // ZAMIN_C_API_H
