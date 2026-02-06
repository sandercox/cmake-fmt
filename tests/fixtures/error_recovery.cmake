# Valid command
message(hello)
# Missing closing paren
set(MY_VAR "value"
# Another valid command after error
project(MyProject)
# Unexpected token
!!!invalid
# Valid again
add_executable(app main.cpp)
