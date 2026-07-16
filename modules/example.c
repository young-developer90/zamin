#include "zamin.h"
#include <string.h>

static ZaminValue add(int argc, const ZaminValue* args) {
    if (argc < 2) {
        ZaminValue r;
        r.tag = ZAMIN_NIL;
        return r;
    }
    long long a = (args[0].tag == ZAMIN_INT) ? args[0].data.as_int : 0;
    long long b = (args[1].tag == ZAMIN_INT) ? args[1].data.as_int : 0;
    ZaminValue r;
    r.tag = ZAMIN_INT;
    r.data.as_int = a + b;
    return r;
}

static ZaminValue greet(int argc, const ZaminValue* args) {
    const char* name = (argc > 0 && args[0].tag == ZAMIN_STRING) ? (const char*)args[0].data.as_str.ptr : "world";
    int len = (argc > 0 && args[0].tag == ZAMIN_STRING) ? (int)args[0].data.as_str.len : 5;
    ZaminValue r;
    r.tag = ZAMIN_NIL;
    return r;
}

static ZaminModuleFunc funcs[] = {
    {"add", add},
    {"greet", greet},
};

int zamin_module_init(int* out_count, ZaminModuleFunc** out_funcs) {
    *out_count = sizeof(funcs) / sizeof(funcs[0]);
    *out_funcs = funcs;
    return 0;
}
