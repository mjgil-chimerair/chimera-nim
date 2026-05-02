## This is a module doc comment
## It describes the module

import std/[strutils, sequtils]

type
  MyEnum = enum
    itemA
    itemB

  MyObject = object
    field*: int
    other: string

const
  Value = 42
  Name = "test"

proc main() =
  ## Main procedure
  echo "Hello, World!"

when isMainModule:
  main()