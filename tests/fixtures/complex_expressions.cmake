# Deeply nested generator expressions
target_compile_definitions(mylib PRIVATE
  $<$<AND:$<BOOL:${ENABLE_FEATURE}>,$<OR:$<CONFIG:Debug>,$<CONFIG:RelWithDebInfo>>>:FEATURE_DEBUG>
)

# Multiple variable reference types
set(RESULT "${MY_VAR}_$ENV{HOME}_${PREFIX_${SUFFIX}}")
message(STATUS "Path: $ENV{PATH}")
message(STATUS "Cache: $CACHE{MY_SETTING}")

# Bracket arguments with different delimiter lengths
message([=[
This is a bracket argument
that spans multiple lines
and can contain ]] without issues
]=])

message([==[
Even ]=] is fine here
]==])

# Generator expression with variable ref inside
target_compile_options(foo PRIVATE
  $<$<BOOL:${USE_SANITIZER}>:-fsanitize=address>
  $<$<AND:$<BOOL:${COVERAGE}>,$<CONFIG:Debug>>:--coverage>
)

# Quoted string with escapes and embedded refs
set(COMPLEX_STRING "Hello \"World\" from ${PROJECT_NAME}\nVersion: ${PROJECT_VERSION}")
