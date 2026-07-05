#include "lion.h"
#include <string.h>

static LionValue add(int argc, const LionValue* args) {
    if (argc < 2) {
        LionValue r;
        r.tag = LION_NIL;
        return r;
    }
    long long a = (args[0].tag == LION_INT) ? args[0].data.as_int : 0;
    long long b = (args[1].tag == LION_INT) ? args[1].data.as_int : 0;
    LionValue r;
    r.tag = LION_INT;
    r.data.as_int = a + b;
    return r;
}

static LionValue greet(int argc, const LionValue* args) {
    const char* name = (argc > 0 && args[0].tag == LION_STRING) ? (const char*)args[0].data.as_str.ptr : "world";
    int len = (argc > 0 && args[0].tag == LION_STRING) ? (int)args[0].data.as_str.len : 5;
    LionValue r;
    r.tag = LION_NIL;
    return r;
}

static LionModuleFunc funcs[] = {
    {"add", add},
    {"greet", greet},
};

int lion_module_init(int* out_count, LionModuleFunc** out_funcs) {
    *out_count = sizeof(funcs) / sizeof(funcs[0]);
    *out_funcs = funcs;
    return 0;
}
