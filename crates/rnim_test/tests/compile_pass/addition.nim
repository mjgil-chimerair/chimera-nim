# Compile-pass fixture
# This file should compile and run successfully

proc add*(a, b: int): int =
  a + b

when isMainModule:
  let x = add(3, 4)
  assert x == 7
  echo "add(3, 4) = ", x