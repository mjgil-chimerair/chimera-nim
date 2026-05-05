# C Backend (`rnim_codegen_c`)

## Output Structure
```c
// Header (.h)
typedef struct { ... } Foo;
void foo(Foo* x);

// Implementation (.c)
#include "generated.h"
void foo(Foo* x) { ... }
```

## Type Mapping
| Nim Type | C Type |
|----------|--------|
| int | int64_t or int |
| float | double |
| string | NimString* |
| seq | NimSeq |
| object | struct |
| ref | struct* |
| cstring | const char* |

## Calling Conventions
- Nimcall: Callee cleans stack
- Cdecl: Caller cleans stack
- Stdcall: Windows stdcall

## FFI Pragmas
```nim
proc myFunc(x: cint) {.importc: "my_c_func", dynlib: "mylib.so".}
proc myFunc(x: cint) {.exportc: "my_c_func".}
```