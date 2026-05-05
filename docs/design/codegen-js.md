# JavaScript Backend (`rnim_codegen_js`)

## Supported Subset
- Integer and float arithmetic
- String operations
- Array/sequence operations
- Objects (ES6 classes)
- Closures
- try/catch/finally
- Async/await for async procedures

## Type Mapping
| Nim Type | JS Type |
|----------|---------|
| int | number |
| float | number |
| string | string |
| seq | Array |
| object | object/class |
| ref | object |

## Unsupported Features
- `goto` statements
- Pointer arithmetic
- Manual memory management
- Threading primitives

## Runtime Shims
- String concatenation: `nimCat`
- Sequence creation: `nimNewSeq`
- Object creation: `nimNewObj`
- Exception handling: `nimThrow`, `nimCatch`