message()
message(   )
set(A)
SET()
add_executable(app)
message(${A}${B}$<CONFIG:Debug>"quoted")
set(HELP [[
Usage: myapp [options]
  --help     Show help
]])
message([=[contains ]] brackets]=])
set(LIST_VAR "a;b;c")
message("line1\nline2\ttab")
