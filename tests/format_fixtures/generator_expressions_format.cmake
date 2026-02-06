target_compile_options(myapp PRIVATE
  $<$<CONFIG:Debug>:-g -O0>
  $<$<CONFIG:Release>:-O3>
  $<$<CXX_COMPILER_ID:MSVC>:/W4 /WX>
  $<$<NOT:$<CXX_COMPILER_ID:MSVC>>:-Wall -Wextra -Wpedantic>
)
target_include_directories(mylib
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
  PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
)
target_compile_definitions(myapp PRIVATE $<$<BOOL:${ENABLE_FEATURE}>:FEATURE_ENABLED>)
set(FLAGS $<$<AND:$<BOOL:${VAR}>,$<OR:$<CONFIG:Debug>,$<CONFIG:RelWithDebInfo>>>:-g>)
