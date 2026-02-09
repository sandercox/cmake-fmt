# Small CMake test file (~100 lines)
# Generated for performance benchmarking

cmake_minimum_required(VERSION 3.20)
project(SmallProject VERSION 1.0.0 LANGUAGES CXX)

# Configuration options
option(BUILD_TESTS "Build test suite" ON)
option(ENABLE_WARNINGS "Enable compiler warnings" ON)

# Find required packages
find_package(Threads REQUIRED)
find_package(OpenSSL REQUIRED)
find_package(ZLIB REQUIRED)

# Set compiler flags
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(ENABLE_WARNINGS)
    set(WARNING_FLAGS "-Wall -Wextra -Wpedantic")
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${WARNING_FLAGS}")
endif()

# Define library target
add_library(small_lib
    src/main.cpp
    src/util.cpp
    src/parser.cpp
    src/formatter.cpp
    include/small_lib.h
)

target_include_directories(small_lib
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_link_libraries(small_lib
    PUBLIC
        Threads::Threads
        OpenSSL::SSL
        OpenSSL::Crypto
    PRIVATE
        ZLIB::ZLIB
)

target_compile_features(small_lib PUBLIC cxx_std_17)

# Conditional test building
if(BUILD_TESTS)
    enable_testing()
    add_executable(small_test tests/test_main.cpp tests/test_util.cpp)
    target_link_libraries(small_test PRIVATE small_lib)
    add_test(NAME small_test COMMAND small_test)
else()
    message(STATUS "Tests disabled")
endif()

# Define executable target
add_executable(small_app
    apps/main.cpp
    apps/cli.cpp
)

target_link_libraries(small_app PRIVATE small_lib)

# Installation rules
install(TARGETS small_lib small_app
    EXPORT small_lib-targets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
    INCLUDES DESTINATION include
)

install(DIRECTORY include/
    DESTINATION include
    FILES_MATCHING PATTERN "*.h"
)

install(EXPORT small_lib-targets
    FILE small_lib-targets.cmake
    NAMESPACE SmallLib::
    DESTINATION lib/cmake/small_lib
)

# Package configuration
include(CMakePackageConfigHelpers)

configure_package_config_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/cmake/small_lib-config.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/small_lib-config.cmake
    INSTALL_DESTINATION lib/cmake/small_lib
)

write_basic_package_version_file(
    ${CMAKE_CURRENT_BINARY_DIR}/small_lib-config-version.cmake
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/small_lib-config.cmake
    ${CMAKE_CURRENT_BINARY_DIR}/small_lib-config-version.cmake
    DESTINATION lib/cmake/small_lib
)
