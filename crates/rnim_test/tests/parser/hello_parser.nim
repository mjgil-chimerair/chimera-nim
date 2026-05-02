# Test fixture for parser category
# This file should parse successfully

proc hello*(): string =
  result = "Hello, World!"

when isMainModule:
  echo hello()