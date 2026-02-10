cmake_minimum_required(VERSION 3.20)
project(MyProject)

# Define a user function with CamelCase
function(MyHelper arg)
  message(STATUS ${arg})
endfunction()

# Define a macro with CamelCase
macro(MyHelper_Macro target)
  add_test(NAME ${target} COMMAND ${target})
endmacro()

# Call builtins (should be lowercased)
SET(X y)
MESSAGE(STATUS "hello")

# Call user-defined commands (should preserve casing via infer)
MyHelper(foo)
MyHelper_Macro(mytest)

# Call unknown user command (no definition found, leave as-is)
SomeExternalCommand(arg1 arg2)
