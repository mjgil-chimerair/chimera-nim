# Diagnostics Design

## Diagnostic Structure

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub file: FileId,
    pub span: Span,
    pub severity: Severity,
    pub code: String,
    pub message: String,
    pub labels: Vec<Label>,
    pub suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub enum Severity {
    Error,
    Warning,
    Note,
    Help,
}

#[derive(Debug, Clone, Serialize)]
pub struct Label {
    pub span: Span,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Suggestion {
    pub span: Span,
    pub message: String,
    pub fix: Option<String>,
}
```

## Diagnostic Codes

| Code | Category | Description |
|------|----------|-------------|
| E0001 | Lexer | Invalid character |
| E0002 | Lexer | Unterminated string |
| E0003 | Lexer | Invalid number literal |
| E0004 | Parser | Expected token |
| E0005 | Parser | Unexpected token |
| E0006 | Parser | Unterminated block |
| E0007 | Sema | Undefined identifier |
| E0008 | Sema | Type mismatch |
| E0009 | Sema | Invalid assignment |
| E0010 | Sema | Missing return |
| E0011 | Sema | Duplicate definition |

## Emitter

The `Emitter` handles diagnostic output:

```rust
pub struct Emitter {
    color: bool,
    json: bool,
}

impl Emitter {
    pub fn new(color: bool, json: bool) -> Self;
    pub fn emit(&self, diag: &Diagnostic);
}
```

## Output Formats

### Human-readable (default)

```
Error: unterminated string literal
  --> test.nim:1:5
  |
1 | "hello
  |      ^ missing closing quote
```

### JSON

```json
{
  "file": 0,
  "span": {"file": 0, "start": 0, "end": 6},
  "severity": "Error",
  "code": "E0002",
  "message": "unterminated string literal",
  "labels": [
    {"span": {"file": 0, "start": 0, "end": 6}, "message": "missing closing quote"}
  ],
  "suggestions": []
}
```

## Fallback Behavior

On panic, the emitter must:
1. Not allocate (avoid double-panic)
2. Write minimal diagnostic to stderr
3. Exit with non-zero code
