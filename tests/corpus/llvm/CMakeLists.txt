# LLVM-style CMakeLists.txt - Complex top-level project configuration
# This file demonstrates LLVM's typical patterns: extensive options, generator expressions,
# PUBLIC/PRIVATE linking, and complex conditional logic
cmake_minimum_required(VERSION 3.20.0)

if(POLICY CMP0068)
  cmake_policy(SET CMP0068 NEW)
endif()

if(POLICY CMP0075)
  cmake_policy(SET CMP0075 NEW)
endif()

project(LLVM VERSION 17.0.6 LANGUAGES C CXX ASM)

# Build options - LLVM has dozens of these
option(
  LLVM_ENABLE_PROJECTS
  "Semicolon-separated list of projects to build"
  "all"
)
option(LLVM_ENABLE_RTTI "Build LLVM with run-time type information" OFF)
option(LLVM_ENABLE_EH "Build LLVM with exception handling" OFF)
option(LLVM_BUILD_RUNTIME "Build the LLVM runtime libraries" ON)
option(LLVM_BUILD_EXAMPLES "Build the LLVM example programs" OFF)
option(LLVM_INCLUDE_EXAMPLES "Generate build targets for the LLVM examples" ON)
option(LLVM_BUILD_TESTS "Build LLVM unit tests" ON)
option(LLVM_INCLUDE_TESTS "Generate build targets for the LLVM unit tests" ON)
option(LLVM_BUILD_BENCHMARKS "Build the LLVM benchmarks" OFF)
option(
  LLVM_INCLUDE_BENCHMARKS
  "Generate build targets for the LLVM benchmarks"
  ON
)
option(LLVM_BUILD_TOOLS "Build the LLVM tools" ON)
option(LLVM_INCLUDE_TOOLS "Generate build targets for the LLVM tools" ON)
option(LLVM_BUILD_UTILS "Build LLVM utility binaries" ON)
option(LLVM_INCLUDE_UTILS "Generate build targets for the LLVM utilities" ON)
option(
  LLVM_ENABLE_ZLIB
  "Use zlib for compression/decompression if available"
  ON
)
option(LLVM_ENABLE_LIBXML2 "Use libxml2 if available" ON)

# Platform-specific configuration
if(WIN32)
  set(LLVM_ON_WIN32 1)
  set(LLVM_ON_UNIX 0)
  if(CYGWIN)
    set(LLVM_ON_WIN32 0)
    set(LLVM_ON_UNIX 1)
  endif()
elseif(UNIX)
  set(LLVM_ON_WIN32 0)
  set(LLVM_ON_UNIX 1)
  if(APPLE)
    set(LLVM_HAVE_LINK_VERSION_SCRIPT 0)
  else()
    set(LLVM_HAVE_LINK_VERSION_SCRIPT 1)
  endif()
else()
  message(FATAL_ERROR "Unknown platform")
endif()

# Set default build type
if(NOT CMAKE_BUILD_TYPE AND NOT CMAKE_CONFIGURATION_TYPES)
  message(STATUS "No build type selected, default to Debug")
  set(CMAKE_BUILD_TYPE "Debug" CACHE STRING "Build type (default Debug)" FORCE)
endif()

# Configure install directories
set(LLVM_INSTALL_BINDIR "bin" CACHE STRING "Path for binary subdirectory")
set(
  LLVM_INSTALL_LIBDIR
  "lib${LLVM_LIBDIR_SUFFIX}"
  CACHE
  STRING
  "Path for library subdirectory"
)
set(
  LLVM_INSTALL_INCLUDEDIR
  "include"
  CACHE
  STRING
  "Path for include subdirectory"
)
set(LLVM_INSTALL_DATADIR "share/llvm" CACHE STRING "Path for data subdirectory")

# Core libraries with PUBLIC/PRIVATE linking (typical LLVM pattern)
add_library(
  LLVMSupport
  STATIC
    lib/Support/APFloat.cpp
    lib/Support/APInt.cpp
    lib/Support/CommandLine.cpp
    lib/Support/Error.cpp
    lib/Support/FileSystem.cpp
    lib/Support/MemoryBuffer.cpp
    lib/Support/Path.cpp
    lib/Support/Process.cpp
    lib/Support/raw_ostream.cpp
    lib/Support/Signals.cpp
    lib/Support/SourceMgr.cpp
    lib/Support/StringExtras.cpp
    lib/Support/TargetParser.cpp
    lib/Support/Timer.cpp
    lib/Support/Triple.cpp
)

target_include_directories(
  LLVMSupport
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}/include>
    $<INSTALL_INTERFACE:include>
  PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    /lib/Support
)

target_compile_definitions(
  LLVMSupport
  PUBLIC
    LLVM_VERSION_MAJOR=${LLVM_VERSION_MAJOR}
    LLVM_VERSION_MINOR=${LLVM_VERSION_MINOR}
    LLVM_VERSION_PATCH=${LLVM_VERSION_PATCH}
    $<$<CONFIG:Debug>:LLVM_ENABLE_DEBUG>
  PRIVATE
    LLVM_BUILD_STATIC
)

# Conditional features with generator expressions
target_compile_options(
  LLVMSupport
  PRIVATE
    $<$<CXX_COMPILER_ID:MSVC>:/W4 /WX>
    $<$<CXX_COMPILER_ID:GNU>:-Wall -Wextra -Wpedantic>
    $<$<CXX_COMPILER_ID:Clang>:-Wall -Wextra -Wpedantic>
    $<$<AND:$<CXX_COMPILER_ID:GNU>,$<CONFIG:Release>>:-O3 -DNDEBUG>
    $<$<AND:$<CXX_COMPILER_ID:Clang>,$<CONFIG:Release>>:-O3 -DNDEBUG>
)

target_link_libraries(
  LLVMSupport
  PUBLIC
    $<$<PLATFORM_ID:Linux>:pthread>
    $<$<PLATFORM_ID:Linux>:dl>
    $<$<PLATFORM_ID:Linux>:rt>
  PRIVATE
    $<$<BOOL:${LLVM_ENABLE_ZLIB}>:ZLIB::ZLIB>
    $<$<BOOL:${LLVM_ENABLE_LIBXML2}>:LibXml2::LibXml2>
)

# Another core library demonstrating dependency chains
add_library(
  LLVMCore
  STATIC
    lib/IR/AsmWriter.cpp
    lib/IR/Attributes.cpp
    lib/IR/BasicBlock.cpp
    lib/IR/Constants.cpp
    lib/IR/Function.cpp
    lib/IR/Instruction.cpp
    lib/IR/Module.cpp
    lib/IR/Type.cpp
    lib/IR/Value.cpp
    lib/IR/Verifier.cpp
)

target_include_directories(
  LLVMCore
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/include>
    $<INSTALL_INTERFACE:include>
  PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}
    /lib/IR
)

target_link_libraries(LLVMCore PUBLIC LLVMSupport PRIVATE LLVMDemangle)

# Tool binary
add_executable(llvm-config tools/llvm-config/llvm-config.cpp)

target_link_libraries(llvm-config PRIVATE LLVMSupport)

# Installation rules with COMPONENT and DESTINATION
install(

  TARGETS
    LLVMSupport
    LLVMCore
    EXPORT
    LLVMExports
  ARCHIVE
  DESTINATION
    ${LLVM_INSTALL_LIBDIR}
    COMPONENT
    llvm-libraries
  LIBRARY
  DESTINATION
    ${LLVM_INSTALL_LIBDIR}
    COMPONENT
    llvm-libraries
  RUNTIME
  DESTINATION
    ${LLVM_INSTALL_BINDIR}
    COMPONENT
    llvm-runtime
)

install(

  TARGETS
    llvm-config
  RUNTIME
  DESTINATION
    ${LLVM_INSTALL_BINDIR}
    COMPONENT
    llvm-tools
)

install(

  DIRECTORY
    include/llvm
  DESTINATION
    ${LLVM_INSTALL_INCLUDEDIR}
    COMPONENT
    llvm-headers
    FILES_MATCHING
    PATTERN
    "*.h"
    PATTERN
    "*.inc"
    PATTERN
    "*.def"
    PATTERN
    ".svn"
    EXCLUDE
)

install(
  EXPORT
  LLVMExports
  FILE
  LLVMExports.cmake
  NAMESPACE
  LLVM::
  DESTINATION
    ${LLVM_INSTALL_LIBDIR}
    /cmake/llvm
    COMPONENT
    llvm-cmake-exports
)

# Complex conditional configuration for different platforms
if(LLVM_BUILD_TESTS)
  if(
  EXISTS
  ${CMAKE_CURRENT_SOURCE_DIR}
  /utils/unittest/googletest/include/gtest/gtest.h
)
    add_subdirectory(utils/unittest)
  else()
    message(WARNING "Could not find Google Test, skipping tests")
  endif()

  if(LLVM_INCLUDE_TESTS)
    add_subdirectory(unittests)
    add_subdirectory(test)
  endif()
endif()

# Version file generation
configure_file(
  ${CMAKE_CURRENT_SOURCE_DIR}
  /include/llvm/Config/llvm-config.h.cmake
  ${CMAKE_CURRENT_BINARY_DIR}
  /include/llvm/Config/llvm-config.h
)

# Export for build tree
export(
  EXPORT
  LLVMExports
  FILE
  "${CMAKE_CURRENT_BINARY_DIR}/cmake/LLVMExports.cmake"
  NAMESPACE
  LLVM::
)

# Package config
include(CMakePackageConfigHelpers)
write_basic_package_version_file(
  "${CMAKE_CURRENT_BINARY_DIR}/LLVMConfigVersion.cmake"
  VERSION
  ${LLVM_VERSION}
  COMPATIBILITY
  AnyNewerVersion
)
