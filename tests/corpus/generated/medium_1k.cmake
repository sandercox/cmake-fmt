# Medium CMake test file (~1000 lines)
# Generated for performance benchmarking

cmake_minimum_required(VERSION 3.20)
project(MediumProject VERSION 2.5.1 LANGUAGES CXX C)

# ============================================================================
# Project Configuration
# ============================================================================

option(BUILD_SHARED_LIBS "Build shared libraries" ON)
option(BUILD_TESTS "Build test suite" ON)
option(BUILD_EXAMPLES "Build example programs" OFF)
option(BUILD_DOCUMENTATION "Build API documentation" OFF)
option(ENABLE_WARNINGS "Enable compiler warnings" ON)
option(ENABLE_WERROR "Treat warnings as errors" OFF)
option(ENABLE_ASAN "Enable address sanitizer" OFF)
option(ENABLE_TSAN "Enable thread sanitizer" OFF)
option(ENABLE_UBSAN "Enable undefined behavior sanitizer" OFF)
option(ENABLE_COVERAGE "Enable code coverage" OFF)
option(ENABLE_LTO "Enable link-time optimization" OFF)
option(USE_CCACHE "Use ccache for compilation" ON)

# ============================================================================
# Compiler Settings
# ============================================================================

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_C_EXTENSIONS OFF)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_POSITION_INDEPENDENT_CODE ON)

if(USE_CCACHE)
    find_program(CCACHE_PROGRAM ccache)
    if(CCACHE_PROGRAM)
        set(CMAKE_CXX_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
        set(CMAKE_C_COMPILER_LAUNCHER "${CCACHE_PROGRAM}")
        message(STATUS "Using ccache: ${CCACHE_PROGRAM}")
    endif()
endif()

# ============================================================================
# Find Dependencies
# ============================================================================

find_package(Threads REQUIRED)
find_package(OpenSSL REQUIRED)
find_package(ZLIB REQUIRED)
find_package(BZip2)
find_package(CURL REQUIRED)
find_package(SQLite3 REQUIRED)
find_package(Boost 1.70 REQUIRED COMPONENTS system filesystem thread program_options)
find_package(Protobuf REQUIRED)
find_package(gRPC CONFIG)

if(NOT gRPC_FOUND)
    find_package(PkgConfig REQUIRED)
    pkg_check_modules(GRPC REQUIRED grpc++)
endif()

if(BUILD_TESTS)
    find_package(GTest REQUIRED)
    find_package(benchmark REQUIRED)
endif()

if(BUILD_DOCUMENTATION)
    find_package(Doxygen REQUIRED)
endif()

# ============================================================================
# Compiler Flags
# ============================================================================

set(COMMON_WARNING_FLAGS "-Wall -Wextra -Wpedantic -Wshadow -Wcast-align -Wunused -Wconversion -Wsign-conversion -Wnull-dereference -Wdouble-promotion -Wformat=2")

if(CMAKE_CXX_COMPILER_ID MATCHES "GNU")
    set(CXX_WARNING_FLAGS "${COMMON_WARNING_FLAGS} -Wduplicated-cond -Wduplicated-branches -Wlogical-op -Wuseless-cast")
elseif(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
    set(CXX_WARNING_FLAGS "${COMMON_WARNING_FLAGS} -Wmost -Wthread-safety -Wloop-analysis")
elseif(MSVC)
    set(CXX_WARNING_FLAGS "/W4 /permissive-")
endif()

if(ENABLE_WARNINGS)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${CXX_WARNING_FLAGS}")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${COMMON_WARNING_FLAGS}")
endif()

if(ENABLE_WERROR)
    if(MSVC)
        set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /WX")
    else()
        set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Werror")
    endif()
endif()

if(ENABLE_ASAN)
    set(SANITIZER_FLAGS "${SANITIZER_FLAGS} -fsanitize=address -fno-omit-frame-pointer")
endif()

if(ENABLE_TSAN)
    set(SANITIZER_FLAGS "${SANITIZER_FLAGS} -fsanitize=thread")
endif()

if(ENABLE_UBSAN)
    set(SANITIZER_FLAGS "${SANITIZER_FLAGS} -fsanitize=undefined")
endif()

if(SANITIZER_FLAGS)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${SANITIZER_FLAGS}")
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} ${SANITIZER_FLAGS}")
endif()

if(ENABLE_COVERAGE)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} --coverage -fprofile-arcs -ftest-coverage")
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} --coverage")
endif()

if(ENABLE_LTO)
    include(CheckIPOSupported)
    check_ipo_supported(RESULT LTO_SUPPORTED OUTPUT LTO_ERROR)
    if(LTO_SUPPORTED)
        set(CMAKE_INTERPROCEDURAL_OPTIMIZATION ON)
        message(STATUS "Link-time optimization enabled")
    else()
        message(WARNING "LTO not supported: ${LTO_ERROR}")
    endif()
endif()

# ============================================================================
# Core Library
# ============================================================================

add_library(medium_core
    src/core/engine.cpp
    src/core/config.cpp
    src/core/logger.cpp
    src/core/error.cpp
    src/core/utils.cpp
    src/core/memory.cpp
    src/core/threading.cpp
    src/core/event_loop.cpp
    include/medium/core/engine.h
    include/medium/core/config.h
    include/medium/core/logger.h
    include/medium/core/error.h
    include/medium/core/utils.h
    include/medium/core/memory.h
    include/medium/core/threading.h
    include/medium/core/event_loop.h
)

target_include_directories(medium_core
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
        ${CMAKE_CURRENT_BINARY_DIR}/generated
)

target_link_libraries(medium_core
    PUBLIC
        Threads::Threads
        Boost::system
        Boost::filesystem
        Boost::thread
    PRIVATE
        OpenSSL::SSL
        OpenSSL::Crypto
        ZLIB::ZLIB
)

target_compile_features(medium_core PUBLIC cxx_std_17)
target_compile_definitions(medium_core PRIVATE MEDIUM_CORE_BUILDING)

if(BUILD_SHARED_LIBS)
    target_compile_definitions(medium_core PUBLIC MEDIUM_SHARED_LIB)
endif()

set_target_properties(medium_core PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    OUTPUT_NAME "medium_core"
    DEBUG_POSTFIX "d"
)

# ============================================================================
# Network Library
# ============================================================================

add_library(medium_network
    src/network/client.cpp
    src/network/server.cpp
    src/network/connection.cpp
    src/network/protocol.cpp
    src/network/ssl_context.cpp
    src/network/websocket.cpp
    include/medium/network/client.h
    include/medium/network/server.h
    include/medium/network/connection.h
    include/medium/network/protocol.h
    include/medium/network/ssl_context.h
    include/medium/network/websocket.h
)

target_include_directories(medium_network
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_link_libraries(medium_network
    PUBLIC
        medium_core
        CURL::libcurl
    PRIVATE
        OpenSSL::SSL
        OpenSSL::Crypto
)

target_compile_features(medium_network PUBLIC cxx_std_17)

set_target_properties(medium_network PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    OUTPUT_NAME "medium_network"
    DEBUG_POSTFIX "d"
)

# ============================================================================
# Database Library
# ============================================================================

add_library(medium_database
    src/database/connection.cpp
    src/database/query.cpp
    src/database/transaction.cpp
    src/database/schema.cpp
    src/database/migration.cpp
    include/medium/database/connection.h
    include/medium/database/query.h
    include/medium/database/transaction.h
    include/medium/database/schema.h
    include/medium/database/migration.h
)

target_include_directories(medium_database
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_link_libraries(medium_database
    PUBLIC
        medium_core
        SQLite::SQLite3
    PRIVATE
        $<$<BOOL:${BZip2_FOUND}>:BZip2::BZip2>
)

target_compile_features(medium_database PUBLIC cxx_std_17)

set_target_properties(medium_database PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    OUTPUT_NAME "medium_database"
    DEBUG_POSTFIX "d"
)

# ============================================================================
# RPC Library
# ============================================================================

set(PROTO_FILES
    proto/service.proto
    proto/message.proto
    proto/types.proto
)

foreach(PROTO_FILE ${PROTO_FILES})
    get_filename_component(PROTO_NAME ${PROTO_FILE} NAME_WE)
    set(PROTO_SRCS ${PROTO_SRCS} ${CMAKE_CURRENT_BINARY_DIR}/generated/${PROTO_NAME}.pb.cc)
    set(PROTO_HDRS ${PROTO_HDRS} ${CMAKE_CURRENT_BINARY_DIR}/generated/${PROTO_NAME}.pb.h)
    set(GRPC_SRCS ${GRPC_SRCS} ${CMAKE_CURRENT_BINARY_DIR}/generated/${PROTO_NAME}.grpc.pb.cc)
    set(GRPC_HDRS ${GRPC_HDRS} ${CMAKE_CURRENT_BINARY_DIR}/generated/${PROTO_NAME}.grpc.pb.h)
endforeach()

add_library(medium_rpc
    src/rpc/service.cpp
    src/rpc/client.cpp
    src/rpc/server.cpp
    ${PROTO_SRCS}
    ${GRPC_SRCS}
    include/medium/rpc/service.h
    include/medium/rpc/client.h
    include/medium/rpc/server.h
)

target_include_directories(medium_rpc
    PUBLIC
        $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
        $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/generated>
        $<INSTALL_INTERFACE:include>
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_link_libraries(medium_rpc
    PUBLIC
        medium_core
        medium_network
        protobuf::libprotobuf
    PRIVATE
        ${GRPC_LIBRARIES}
)

target_compile_features(medium_rpc PUBLIC cxx_std_17)

set_target_properties(medium_rpc PROPERTIES
    VERSION ${PROJECT_VERSION}
    SOVERSION ${PROJECT_VERSION_MAJOR}
    OUTPUT_NAME "medium_rpc"
    DEBUG_POSTFIX "d"
)

# ============================================================================
# Utility Functions and Macros
# ============================================================================

function(add_medium_executable TARGET_NAME)
    cmake_parse_arguments(ARG "" "" "SOURCES;DEPENDS" ${ARGN})

    add_executable(${TARGET_NAME} ${ARG_SOURCES})

    target_link_libraries(${TARGET_NAME}
        PRIVATE
            medium_core
            ${ARG_DEPENDS}
    )

    if(ENABLE_WARNINGS)
        target_compile_options(${TARGET_NAME} PRIVATE ${CXX_WARNING_FLAGS})
    endif()

    set_target_properties(${TARGET_NAME} PROPERTIES
        RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin
    )
endfunction()

macro(add_medium_test TEST_NAME)
    if(BUILD_TESTS)
        add_executable(${TEST_NAME} ${ARGN})
        target_link_libraries(${TEST_NAME}
            PRIVATE
                medium_core
                GTest::gtest
                GTest::gtest_main
        )
        add_test(NAME ${TEST_NAME} COMMAND ${TEST_NAME})
    endif()
endmacro()

function(add_medium_benchmark BENCH_NAME)
    cmake_parse_arguments(ARG "" "" "SOURCES;DEPENDS" ${ARGN})

    if(BUILD_TESTS)
        add_executable(${BENCH_NAME} ${ARG_SOURCES})

        target_link_libraries(${BENCH_NAME}
            PRIVATE
                medium_core
                benchmark::benchmark
                benchmark::benchmark_main
                ${ARG_DEPENDS}
        )

        set_target_properties(${BENCH_NAME} PROPERTIES
            RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/benchmarks
        )
    endif()
endfunction()

# ============================================================================
# Main Application
# ============================================================================

add_medium_executable(medium_app
    SOURCES
        apps/main.cpp
        apps/cli.cpp
        apps/commands.cpp
        apps/config_loader.cpp
    DEPENDS
        medium_network
        medium_database
        medium_rpc
        Boost::program_options
)

# ============================================================================
# Example Programs
# ============================================================================

if(BUILD_EXAMPLES)
    add_medium_executable(example_client
        SOURCES examples/client_example.cpp
        DEPENDS medium_network medium_rpc
    )

    add_medium_executable(example_server
        SOURCES examples/server_example.cpp
        DEPENDS medium_network medium_rpc
    )

    add_medium_executable(example_database
        SOURCES examples/database_example.cpp
        DEPENDS medium_database
    )

    add_medium_executable(example_threading
        SOURCES examples/threading_example.cpp
        DEPENDS medium_core
    )
endif()

# ============================================================================
# Testing
# ============================================================================

if(BUILD_TESTS)
    enable_testing()

    # Core tests
    add_medium_test(test_engine tests/core/test_engine.cpp)
    target_link_libraries(test_engine PRIVATE medium_core)

    add_medium_test(test_config tests/core/test_config.cpp)
    target_link_libraries(test_config PRIVATE medium_core)

    add_medium_test(test_logger tests/core/test_logger.cpp)
    target_link_libraries(test_logger PRIVATE medium_core)

    add_medium_test(test_memory tests/core/test_memory.cpp)
    target_link_libraries(test_memory PRIVATE medium_core)

    add_medium_test(test_threading tests/core/test_threading.cpp)
    target_link_libraries(test_threading PRIVATE medium_core)

    # Network tests
    add_medium_test(test_client tests/network/test_client.cpp)
    target_link_libraries(test_client PRIVATE medium_network)

    add_medium_test(test_server tests/network/test_server.cpp)
    target_link_libraries(test_server PRIVATE medium_network)

    add_medium_test(test_connection tests/network/test_connection.cpp)
    target_link_libraries(test_connection PRIVATE medium_network)

    add_medium_test(test_websocket tests/network/test_websocket.cpp)
    target_link_libraries(test_websocket PRIVATE medium_network)

    # Database tests
    add_medium_test(test_db_connection tests/database/test_connection.cpp)
    target_link_libraries(test_db_connection PRIVATE medium_database)

    add_medium_test(test_query tests/database/test_query.cpp)
    target_link_libraries(test_query PRIVATE medium_database)

    add_medium_test(test_transaction tests/database/test_transaction.cpp)
    target_link_libraries(test_transaction PRIVATE medium_database)

    add_medium_test(test_migration tests/database/test_migration.cpp)
    target_link_libraries(test_migration PRIVATE medium_database)

    # RPC tests
    add_medium_test(test_rpc_service tests/rpc/test_service.cpp)
    target_link_libraries(test_rpc_service PRIVATE medium_rpc)

    add_medium_test(test_rpc_client tests/rpc/test_client.cpp)
    target_link_libraries(test_rpc_client PRIVATE medium_rpc)

    add_medium_test(test_rpc_server tests/rpc/test_server.cpp)
    target_link_libraries(test_rpc_server PRIVATE medium_rpc)

    # Integration tests
    add_medium_test(integration_test_full_stack tests/integration/test_full_stack.cpp)
    target_link_libraries(integration_test_full_stack PRIVATE
        medium_core
        medium_network
        medium_database
        medium_rpc
    )

    # Benchmarks
    add_medium_benchmark(bench_engine
        SOURCES benchmarks/bench_engine.cpp
        DEPENDS medium_core
    )

    add_medium_benchmark(bench_network
        SOURCES benchmarks/bench_network.cpp
        DEPENDS medium_network
    )

    add_medium_benchmark(bench_database
        SOURCES benchmarks/bench_database.cpp
        DEPENDS medium_database
    )

    add_medium_benchmark(bench_rpc
        SOURCES benchmarks/bench_rpc.cpp
        DEPENDS medium_rpc
    )
endif()

# ============================================================================
# Installation
# ============================================================================

install(TARGETS medium_core medium_network medium_database medium_rpc medium_app
    EXPORT medium-targets
    LIBRARY DESTINATION lib
    ARCHIVE DESTINATION lib
    RUNTIME DESTINATION bin
    INCLUDES DESTINATION include
)

install(DIRECTORY include/medium/
    DESTINATION include/medium
    FILES_MATCHING
    PATTERN "*.h"
    PATTERN "*.hpp"
)

install(DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}/generated/
    DESTINATION include/medium/generated
    FILES_MATCHING
    PATTERN "*.pb.h"
    PATTERN "*.grpc.pb.h"
)

install(EXPORT medium-targets
    FILE medium-targets.cmake
    NAMESPACE Medium::
    DESTINATION lib/cmake/medium
)

# ============================================================================
# CMake Package Configuration
# ============================================================================

include(CMakePackageConfigHelpers)

configure_package_config_file(
    ${CMAKE_CURRENT_SOURCE_DIR}/cmake/medium-config.cmake.in
    ${CMAKE_CURRENT_BINARY_DIR}/medium-config.cmake
    INSTALL_DESTINATION lib/cmake/medium
    PATH_VARS CMAKE_INSTALL_PREFIX
)

write_basic_package_version_file(
    ${CMAKE_CURRENT_BINARY_DIR}/medium-config-version.cmake
    VERSION ${PROJECT_VERSION}
    COMPATIBILITY SameMajorVersion
)

install(FILES
    ${CMAKE_CURRENT_BINARY_DIR}/medium-config.cmake
    ${CMAKE_CURRENT_BINARY_DIR}/medium-config-version.cmake
    DESTINATION lib/cmake/medium
)

# ============================================================================
# Documentation
# ============================================================================

if(BUILD_DOCUMENTATION)
    set(DOXYGEN_EXTRACT_ALL YES)
    set(DOXYGEN_EXTRACT_PRIVATE YES)
    set(DOXYGEN_EXTRACT_STATIC YES)
    set(DOXYGEN_GENERATE_HTML YES)
    set(DOXYGEN_GENERATE_XML YES)
    set(DOXYGEN_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/docs)
    set(DOXYGEN_PROJECT_NAME "Medium Project")
    set(DOXYGEN_PROJECT_VERSION ${PROJECT_VERSION})
    set(DOXYGEN_PROJECT_BRIEF "A medium-sized CMake project for testing")

    doxygen_add_docs(docs
        ${CMAKE_CURRENT_SOURCE_DIR}/include
        ${CMAKE_CURRENT_SOURCE_DIR}/src
        COMMENT "Generating API documentation"
    )
endif()

# ============================================================================
# CPack Configuration
# ============================================================================

set(CPACK_PACKAGE_NAME "MediumProject")
set(CPACK_PACKAGE_VENDOR "Example Vendor")
set(CPACK_PACKAGE_DESCRIPTION_SUMMARY "Medium CMake project for performance testing")
set(CPACK_PACKAGE_VERSION_MAJOR ${PROJECT_VERSION_MAJOR})
set(CPACK_PACKAGE_VERSION_MINOR ${PROJECT_VERSION_MINOR})
set(CPACK_PACKAGE_VERSION_PATCH ${PROJECT_VERSION_PATCH})
set(CPACK_PACKAGE_INSTALL_DIRECTORY "MediumProject")
set(CPACK_RESOURCE_FILE_LICENSE "${CMAKE_CURRENT_SOURCE_DIR}/LICENSE")
set(CPACK_RESOURCE_FILE_README "${CMAKE_CURRENT_SOURCE_DIR}/README.md")

if(WIN32)
    set(CPACK_GENERATOR "ZIP;NSIS")
elseif(APPLE)
    set(CPACK_GENERATOR "TGZ;DragNDrop")
else()
    set(CPACK_GENERATOR "TGZ;DEB;RPM")
endif()

include(CPack)

# ============================================================================
# Platform-specific Settings
# ============================================================================

if(WIN32)
    target_compile_definitions(medium_core PRIVATE PLATFORM_WINDOWS)
    target_compile_definitions(medium_core PRIVATE _WIN32_WINNT=0x0601)
    target_compile_definitions(medium_core PRIVATE NOMINMAX)
    target_compile_definitions(medium_core PRIVATE WIN32_LEAN_AND_MEAN)
elseif(APPLE)
    target_compile_definitions(medium_core PRIVATE PLATFORM_MACOS)
    target_compile_options(medium_core PRIVATE -mmacosx-version-min=10.15)
elseif(UNIX)
    target_compile_definitions(medium_core PRIVATE PLATFORM_LINUX)
    target_link_libraries(medium_core PRIVATE dl rt)
endif()

# ============================================================================
# Build Information
# ============================================================================

message(STATUS "=== Build Configuration ===")
message(STATUS "Project: ${PROJECT_NAME} ${PROJECT_VERSION}")
message(STATUS "Build type: ${CMAKE_BUILD_TYPE}")
message(STATUS "C++ standard: ${CMAKE_CXX_STANDARD}")
message(STATUS "C standard: ${CMAKE_C_STANDARD}")
message(STATUS "Install prefix: ${CMAKE_INSTALL_PREFIX}")
message(STATUS "")
message(STATUS "=== Features ===")
message(STATUS "Shared libraries: ${BUILD_SHARED_LIBS}")
message(STATUS "Tests: ${BUILD_TESTS}")
message(STATUS "Examples: ${BUILD_EXAMPLES}")
message(STATUS "Documentation: ${BUILD_DOCUMENTATION}")
message(STATUS "Warnings: ${ENABLE_WARNINGS}")
message(STATUS "Werror: ${ENABLE_WERROR}")
message(STATUS "Address sanitizer: ${ENABLE_ASAN}")
message(STATUS "Thread sanitizer: ${ENABLE_TSAN}")
message(STATUS "UB sanitizer: ${ENABLE_UBSAN}")
message(STATUS "Coverage: ${ENABLE_COVERAGE}")
message(STATUS "LTO: ${ENABLE_LTO}")
message(STATUS "Ccache: ${USE_CCACHE}")
message(STATUS "")
message(STATUS "=== Dependencies ===")
message(STATUS "Threads: ${CMAKE_THREAD_LIBS_INIT}")
message(STATUS "OpenSSL: ${OPENSSL_VERSION}")
message(STATUS "ZLIB: ${ZLIB_VERSION_STRING}")
message(STATUS "CURL: ${CURL_VERSION_STRING}")
message(STATUS "SQLite3: ${SQLite3_VERSION}")
message(STATUS "Boost: ${Boost_VERSION}")
message(STATUS "Protobuf: ${Protobuf_VERSION}")

if(BUILD_TESTS)
    message(STATUS "GTest: ${GTest_VERSION}")
    message(STATUS "benchmark: found")
endif()

message(STATUS "=== Compiler ===")
message(STATUS "C++ compiler: ${CMAKE_CXX_COMPILER_ID} ${CMAKE_CXX_COMPILER_VERSION}")
message(STATUS "C compiler: ${CMAKE_C_COMPILER_ID} ${CMAKE_C_COMPILER_VERSION}")
message(STATUS "")
