# Empty command
message()

# Command with only whitespace in args
message(   )

# Multiple blank lines


# Tab indentation
	message(	tabbed	)

# Adjacent special constructs (no whitespace between)
message(${A}${B}$<CONFIG:Debug>"quoted")

# Very long single line
target_link_libraries(myapp PRIVATE lib1 lib2 lib3 lib4 lib5 lib6 lib7 lib8 lib9 lib10 lib11 lib12 lib13 lib14 lib15)

# Bracket argument with 0 equals and embedded newlines
set(HELP_TEXT [[
Usage: myapp [options]
  --help     Show help
  --version  Show version
]])

# Line comment immediately after command (no space)
message(hello)# comment right after paren

# Semicolons as argument separators
set(LIST_VAR "a;b;c")
