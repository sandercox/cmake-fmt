# HandleLLVMOptions.cmake - Platform-specific compiler configuration
# Heavy conditional logic, platform detection, compiler flag handling

# This module configures compiler options based on the platform and build settings

include(CheckCCompilerFlag)
include(CheckCXXCompilerFlag)
include(CheckLinkerFlag)

# Detect compiler
if(CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  set(LLVM_COMPILER_IS_CLANG ON)
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "GNU")
  set(LLVM_COMPILER_IS_GCC ON)
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "MSVC")
  set(LLVM_COMPILER_IS_MSVC ON)
elseif(CMAKE_CXX_COMPILER_ID STREQUAL "Intel")
  set(LLVM_COMPILER_IS_ICC ON)
endif()

# Platform detection
if(WIN32)
  if(CYGWIN)
    set(LLVM_ON_UNIX 1)
    set(LLVM_ON_WIN32 0)
  else()
    set(LLVM_ON_WIN32 1)
    set(LLVM_ON_UNIX 0)
  endif()
  set(LLVM_HAVE_LINK_VERSION_SCRIPT 0)
elseif(UNIX)
  set(LLVM_ON_WIN32 0)
  set(LLVM_ON_UNIX 1)
  if(APPLE)
    set(LLVM_HAVE_LINK_VERSION_SCRIPT 0)
  else()
    set(LLVM_HAVE_LINK_VERSION_SCRIPT 1)
  endif()
else()
  message(SEND_ERROR "Unable to determine platform")
endif()

# C++ standard
if(NOT DEFINED CMAKE_CXX_STANDARD)
  set(CMAKE_CXX_STANDARD 17)
endif()
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

# Warning flags
if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
  set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -Wall -Wextra")

  # Additional warnings for high quality code
  set(warning_flags
    -Wcast-qual
    -Wformat=2
    -Wmissing-declarations
    -Wno-long-long
    -Wno-unused-parameter
    -Wwrite-strings
    -Wno-deprecated-declarations)

  foreach(flag ${warning_flags})
    string(REPLACE "-" "_" flag_var ${flag})
    string(REPLACE "=" "_" flag_var ${flag_var})
    string(TOUPPER ${flag_var} flag_var)

    check_cxx_compiler_flag("${flag}" CXX_SUPPORTS${flag_var})
    if(CXX_SUPPORTS${flag_var})
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${flag}")
    endif()

    check_c_compiler_flag("${flag}" C_SUPPORTS${flag_var})
    if(C_SUPPORTS${flag_var})
      set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${flag}")
    endif()
  endforeach()
elseif(LLVM_COMPILER_IS_MSVC)
  # MSVC specific warnings
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /W4")
  set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} /W4")

  # Disable specific MSVC warnings that are too noisy
  set(msvc_warning_flags
    /wd4141  # 'modifier' : used more than once
    /wd4146  # unary minus operator applied to unsigned type
    /wd4244  # 'conversion' conversion from 'type1' to 'type2'
    /wd4267  # 'var' : conversion from 'size_t' to 'type'
    /wd4291  # 'declaration' : no matching operator delete found
    /wd4351  # new behavior: elements of array will be default initialized
    /wd4456  # declaration hides previous local declaration
    /wd4457  # declaration hides function parameter
    /wd4458  # declaration hides class member
    /wd4459  # declaration hides global declaration
    /wd4503  # decorated name length exceeded
    /wd4624  # destructor could not be generated
    /wd4722  # destructor never returns
    /wd4800  # forcing value to bool 'true' or 'false'
    /wd4100  # unreferenced formal parameter
    /wd4127  # conditional expression is constant
    /wd4512  # assignment operator could not be generated
    /wd4505  # unreferenced local function has been removed
    /wd4610  # struct can never be instantiated
    /wd4510  # default constructor could not be generated
    /wd4702  # unreachable code
    /wd4245  # signed/unsigned mismatch
    /wd4706  # assignment within conditional expression
    /wd4310  # cast truncates constant value
    /wd4701  # potentially uninitialized local variable
    /wd4703  # potentially uninitialized local pointer variable
    /wd4389  # signed/unsigned mismatch
    /wd4611  # interaction between '_setjmp' and C++ object destruction
    /wd4805  # unsafe mix of type and type in operation
    /wd4204  # nonstandard extension used
    /wd4577  # 'noexcept' used with no exception handling mode specified
    /wd4091  # 'keyword' : ignored on left of 'type' when no variable is declared
    /wd4592  # symbol will be dynamically initialized
    /wd4319  # zero extending result of unary operation
    /wd4324  # structure was padded due to alignment specifier)

  foreach(flag ${msvc_warning_flags})
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${flag}")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${flag}")
  endforeach()
endif()

# RTTI handling
if(NOT LLVM_ENABLE_RTTI)
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-rtti")
  elseif(LLVM_COMPILER_IS_MSVC)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /GR-")
  endif()
endif()

# Exception handling
if(NOT LLVM_ENABLE_EH)
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fno-exceptions")
  elseif(LLVM_COMPILER_IS_MSVC)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /EHs-c-")
  endif()
endif()

# Optimization flags
if(CMAKE_BUILD_TYPE STREQUAL "Release")
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    set(CMAKE_CXX_FLAGS_RELEASE "-O3 -DNDEBUG")
    set(CMAKE_C_FLAGS_RELEASE "-O3 -DNDEBUG")
  elseif(LLVM_COMPILER_IS_MSVC)
    set(CMAKE_CXX_FLAGS_RELEASE "/O2 /Ob2 /DNDEBUG")
    set(CMAKE_C_FLAGS_RELEASE "/O2 /Ob2 /DNDEBUG")
  endif()
elseif(CMAKE_BUILD_TYPE STREQUAL "Debug")
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    set(CMAKE_CXX_FLAGS_DEBUG "-g -O0")
    set(CMAKE_C_FLAGS_DEBUG "-g -O0")
  elseif(LLVM_COMPILER_IS_MSVC)
    set(CMAKE_CXX_FLAGS_DEBUG "/Od /Zi /RTC1")
    set(CMAKE_C_FLAGS_DEBUG "/Od /Zi /RTC1")
  endif()
elseif(CMAKE_BUILD_TYPE STREQUAL "RelWithDebInfo")
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    set(CMAKE_CXX_FLAGS_RELWITHDEBINFO "-O2 -g -DNDEBUG")
    set(CMAKE_C_FLAGS_RELWITHDEBINFO "-O2 -g -DNDEBUG")
  elseif(LLVM_COMPILER_IS_MSVC)
    set(CMAKE_CXX_FLAGS_RELWITHDEBINFO "/O2 /Zi /DNDEBUG")
    set(CMAKE_C_FLAGS_RELWITHDEBINFO "/O2 /Zi /DNDEBUG")
  endif()
elseif(CMAKE_BUILD_TYPE STREQUAL "MinSizeRel")
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    set(CMAKE_CXX_FLAGS_MINSIZEREL "-Os -DNDEBUG")
    set(CMAKE_C_FLAGS_MINSIZEREL "-Os -DNDEBUG")
  elseif(LLVM_COMPILER_IS_MSVC)
    set(CMAKE_CXX_FLAGS_MINSIZEREL "/O1 /DNDEBUG")
    set(CMAKE_C_FLAGS_MINSIZEREL "/O1 /DNDEBUG")
  endif()
endif()

# Position Independent Code
if(NOT WIN32)
  set(CMAKE_POSITION_INDEPENDENT_CODE ON)
endif()

# LTO (Link Time Optimization)
option(LLVM_ENABLE_LTO "Build LLVM with LTO" OFF)

if(LLVM_ENABLE_LTO)
  if(CMAKE_VERSION VERSION_GREATER_EQUAL 3.9)
    set(CMAKE_INTERPROCEDURAL_OPTIMIZATION TRUE)
  else()
    if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -flto")
      set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -flto")
      set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -flto")
      set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -flto")
    endif()
  endif()
endif()

# Sanitizers
option(LLVM_USE_SANITIZER "Build with sanitizer support" "")

if(LLVM_USE_SANITIZER)
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    if(LLVM_USE_SANITIZER STREQUAL "Address")
      set(SANITIZER_FLAGS "-fsanitize=address -fno-omit-frame-pointer")
    elseif(LLVM_USE_SANITIZER STREQUAL "Thread")
      set(SANITIZER_FLAGS "-fsanitize=thread")
    elseif(LLVM_USE_SANITIZER STREQUAL "Memory")
      set(SANITIZER_FLAGS "-fsanitize=memory -fno-omit-frame-pointer")
    elseif(LLVM_USE_SANITIZER STREQUAL "Undefined")
      set(SANITIZER_FLAGS "-fsanitize=undefined -fno-omit-frame-pointer")
    elseif(LLVM_USE_SANITIZER STREQUAL "Address;Undefined" OR
           LLVM_USE_SANITIZER STREQUAL "Undefined;Address")
      set(SANITIZER_FLAGS "-fsanitize=address,undefined -fno-omit-frame-pointer")
    else()
      message(WARNING "Unsupported sanitizer: ${LLVM_USE_SANITIZER}")
    endif()

    if(SANITIZER_FLAGS)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${SANITIZER_FLAGS}")
      set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} ${SANITIZER_FLAGS}")
      set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} ${SANITIZER_FLAGS}")
    endif()
  else()
    message(WARNING "Sanitizers are only supported with GCC and Clang")
  endif()
endif()

# Coverage
option(LLVM_BUILD_INSTRUMENTED_COVERAGE "Build LLVM with code coverage instrumentation" OFF)

if(LLVM_BUILD_INSTRUMENTED_COVERAGE)
  if(LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} --coverage")
    set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} --coverage")
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} --coverage")
  else()
    message(FATAL_ERROR "Code coverage is only supported with GCC and Clang")
  endif()
endif()

# Linker selection
option(LLVM_USE_LINKER "Use specific linker (lld, gold, bfd)" "")

if(LLVM_USE_LINKER AND (LLVM_COMPILER_IS_GCC OR LLVM_COMPILER_IS_CLANG))
  if(LLVM_USE_LINKER STREQUAL "lld")
    check_linker_flag(CXX "-fuse-ld=lld" CXX_SUPPORTS_LLD)
    if(CXX_SUPPORTS_LLD)
      set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -fuse-ld=lld")
      set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -fuse-ld=lld")
    else()
      message(WARNING "lld linker not supported by compiler")
    endif()
  elseif(LLVM_USE_LINKER STREQUAL "gold")
    check_linker_flag(CXX "-fuse-ld=gold" CXX_SUPPORTS_GOLD)
    if(CXX_SUPPORTS_GOLD)
      set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -fuse-ld=gold")
      set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -fuse-ld=gold")
    else()
      message(WARNING "gold linker not supported by compiler")
    endif()
  elseif(LLVM_USE_LINKER STREQUAL "bfd")
    set(CMAKE_EXE_LINKER_FLAGS "${CMAKE_EXE_LINKER_FLAGS} -fuse-ld=bfd")
    set(CMAKE_SHARED_LINKER_FLAGS "${CMAKE_SHARED_LINKER_FLAGS} -fuse-ld=bfd")
  endif()
endif()

# Export compile commands for tooling
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Color diagnostics
if(LLVM_COMPILER_IS_CLANG)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fcolor-diagnostics")
  set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -fcolor-diagnostics")
elseif(LLVM_COMPILER_IS_GCC)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -fdiagnostics-color=always")
  set(CMAKE_C_FLAGS "${CMAKE_C_FLAGS} -fdiagnostics-color=always")
endif()
