cmake_minimum_required(VERSION 3.20)
project(MyProject LANGUAGES CXX)

set(SOURCES
  main.cpp
  utils.cpp
  helper.cpp
)

add_executable(myapp ${SOURCES})
target_link_libraries(myapp PRIVATE fmt::fmt)
