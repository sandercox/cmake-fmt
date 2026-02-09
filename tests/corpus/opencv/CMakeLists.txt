# OpenCV-style CMakeLists.txt - Computer vision library with complex cross-platform logic
# Demonstrates: extensive platform detection, option chains, conditional targets, complex if() conditions

cmake_minimum_required(VERSION 3.5.1)

# Policy settings for modern CMake
if(POLICY CMP0071)
  cmake_policy(SET CMP0071 NEW)
endif()
if(POLICY CMP0091)
  cmake_policy(SET CMP0091 NEW)
endif()

project(OpenCV CXX C)

set(OPENCV_VERSION_MAJOR 4)
set(OPENCV_VERSION_MINOR 8)
set(OPENCV_VERSION_PATCH 0)
set(OPENCV_VERSION "${OPENCV_VERSION_MAJOR}.${OPENCV_VERSION_MINOR}.${OPENCV_VERSION_PATCH}")

# Platform detection
if(WIN32 AND NOT MINGW)
  set(OPENCV_PLATFORM_WINDOWS ON)
elseif(APPLE)
  set(OPENCV_PLATFORM_APPLE ON)
  if(IOS)
    set(OPENCV_PLATFORM_IOS ON)
  else()
    set(OPENCV_PLATFORM_MACOS ON)
  endif()
elseif(ANDROID)
  set(OPENCV_PLATFORM_ANDROID ON)
elseif(UNIX)
  set(OPENCV_PLATFORM_LINUX ON)
endif()

# Build options - OpenCV has many of these
option(BUILD_SHARED_LIBS "Build shared libraries" ON)
option(BUILD_opencv_apps "Build utility applications" ON)
option(BUILD_opencv_python2 "Build Python 2.x bindings" OFF)
option(BUILD_opencv_python3 "Build Python 3.x bindings" ON)
option(BUILD_DOCS "Create build rules for OpenCV Documentation" OFF)
option(BUILD_EXAMPLES "Build all examples" OFF)
option(BUILD_TESTS "Build all tests" ON)
option(BUILD_PERF_TESTS "Build performance tests" ON)
option(BUILD_WITH_DEBUG_INFO "Include debug info into release binaries" OFF)
option(BUILD_WITH_STATIC_CRT "Build with static CRT linkage (MSVC only)" OFF)
option(BUILD_FAT_JAVA_LIB "Create fat java wrapper" ON)
option(BUILD_ANDROID_EXAMPLES "Build examples for Android platform" ON)
option(BUILD_CUDA_STUBS "Build CUDA modules stubs" OFF)
option(BUILD_JAVA "Enable Java support" ON)

# Performance/optimization options
option(ENABLE_FAST_MATH "Enable -ffast-math (not recommended)" OFF)
option(ENABLE_NEON "Enable NEON instructions" OFF)
option(ENABLE_AVX "Enable AVX instructions" OFF)
option(ENABLE_AVX2 "Enable AVX2 instructions" OFF)
option(ENABLE_SSE41 "Enable SSE4.1 instructions" OFF)
option(ENABLE_SSE42 "Enable SSE4.2 instructions" OFF)

# 3rd party library options
option(WITH_CUDA "Include NVIDIA CUDA Runtime support" OFF)
option(WITH_CUDNN "Include NVIDIA cuDNN support" OFF)
option(WITH_OPENCL "Include OpenCL Runtime support" OFF)
option(WITH_FFMPEG "Include FFMPEG support" ON)
option(WITH_GSTREAMER "Include Gstreamer support" ON)
option(WITH_GTK "Use GTK+ 2.x or 3.x" ON)
option(WITH_QT "Build with Qt Backend support" OFF)
option(WITH_TBB "Include Intel TBB support" ON)
option(WITH_EIGEN "Include Eigen support" ON)
option(WITH_OPENVX "Include OpenVX support" OFF)
option(WITH_OPENMP "Include OpenMP support" OFF)

# Architecture detection
if(CMAKE_SYSTEM_PROCESSOR MATCHES "amd64|x86_64|AMD64")
  set(X86_64 1)
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "i686|i386|x86")
  set(X86 1)
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "^(aarch64.*|AARCH64.*|arm64.*|ARM64.*)")
  set(AARCH64 1)
elseif(CMAKE_SYSTEM_PROCESSOR MATCHES "^(arm.*|ARM.*)")
  set(ARM 1)
endif()

# Compiler configuration
if(MSVC)
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /W4")
  if(BUILD_WITH_STATIC_CRT)
    foreach(flag_var CMAKE_CXX_FLAGS CMAKE_CXX_FLAGS_DEBUG CMAKE_CXX_FLAGS_RELEASE
                     CMAKE_CXX_FLAGS_MINSIZEREL CMAKE_CXX_FLAGS_RELWITHDEBINFO)
      if(${flag_var} MATCHES "/MD")
        string(REGEX REPLACE "/MD" "/MT" ${flag_var} "${${flag_var}}")
      endif()
    endforeach()
  endif()
elseif(CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
  set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wall -Wextra")
  if(ENABLE_FAST_MATH)
    set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -ffast-math")
  endif()
endif()

# SIMD instruction support
if(X86_64 OR X86)
  if(ENABLE_AVX2)
    if(MSVC)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /arch:AVX2")
    else()
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -mavx2")
    endif()
  elseif(ENABLE_AVX)
    if(MSVC)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} /arch:AVX")
    else()
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -mavx")
    endif()
  endif()

  if(ENABLE_SSE42)
    if(NOT MSVC)
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -msse4.2")
    endif()
  endif()
endif()

if(ARM OR AARCH64)
  if(ENABLE_NEON)
    if(CMAKE_COMPILER_IS_GNUCXX OR CMAKE_CXX_COMPILER_ID MATCHES "Clang")
      set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -mfpu=neon")
    endif()
  endif()
endif()

# Find dependencies
if(WITH_CUDA AND NOT OPENCV_PLATFORM_IOS AND NOT OPENCV_PLATFORM_ANDROID)
  find_package(CUDA 9.0)
  if(CUDA_FOUND)
    set(HAVE_CUDA ON)
    if(WITH_CUDNN)
      # cuDNN detection would go here
      set(HAVE_CUDNN OFF)
    endif()
  endif()
endif()

if(WITH_OPENCL)
  find_package(OpenCL)
  if(OpenCL_FOUND)
    set(HAVE_OPENCL ON)
  endif()
endif()

if(WITH_TBB)
  find_package(TBB)
  if(TBB_FOUND)
    set(HAVE_TBB ON)
  endif()
endif()

if(WITH_EIGEN)
  find_package(Eigen3)
  if(Eigen3_FOUND)
    set(HAVE_EIGEN ON)
  endif()
endif()

# GTK support on Linux
if(WITH_GTK AND OPENCV_PLATFORM_LINUX)
  find_package(PkgConfig)
  if(PKG_CONFIG_FOUND)
    pkg_check_modules(GTK3 gtk+-3.0)
    if(GTK3_FOUND)
      set(HAVE_GTK ON)
      set(HAVE_GTK3 ON)
    else()
      pkg_check_modules(GTK2 gtk+-2.0)
      if(GTK2_FOUND)
        set(HAVE_GTK ON)
        set(HAVE_GTK2 ON)
      endif()
    endif()
  endif()
endif()

# Core module
add_library(opencv_core)

target_sources(opencv_core
  PRIVATE
    modules/core/src/matrix.cpp
    modules/core/src/array.cpp
    modules/core/src/alloc.cpp
    modules/core/src/arithm.cpp
    modules/core/src/batch_distance.cpp
    modules/core/src/copy.cpp
    modules/core/src/convert.cpp
    modules/core/src/datastructs.cpp
    modules/core/src/dxt.cpp
    modules/core/src/matmul.cpp
    modules/core/src/matop.cpp
    modules/core/src/mathfuncs.cpp
    modules/core/src/persistence.cpp
    modules/core/src/stat.cpp
    modules/core/src/system.cpp
    modules/core/src/tables.cpp
    modules/core/src/types.cpp
    modules/core/src/umatrix.cpp)

target_include_directories(opencv_core
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/modules/core/include>
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}>
    $<INSTALL_INTERFACE:include/opencv4>)

target_compile_definitions(opencv_core
  PUBLIC
    OPENCV_VERSION_MAJOR=${OPENCV_VERSION_MAJOR}
    OPENCV_VERSION_MINOR=${OPENCV_VERSION_MINOR}
    OPENCV_VERSION_PATCH=${OPENCV_VERSION_PATCH}
  PRIVATE
    $<$<BOOL:${HAVE_CUDA}>:HAVE_CUDA>
    $<$<BOOL:${HAVE_OPENCL}>:HAVE_OPENCL>
    $<$<BOOL:${HAVE_TBB}>:HAVE_TBB>)

# Conditional linking based on available libraries
target_link_libraries(opencv_core
  PUBLIC
    $<$<PLATFORM_ID:Linux>:pthread>
    $<$<PLATFORM_ID:Linux>:dl>
  PRIVATE
    $<$<BOOL:${HAVE_TBB}>:TBB::tbb>
    $<$<AND:$<BOOL:${WITH_OPENMP}>,$<NOT:$<BOOL:${HAVE_TBB}>>>:OpenMP::OpenMP_CXX>)

set_target_properties(opencv_core PROPERTIES
  VERSION ${OPENCV_VERSION}
  OUTPUT_NAME opencv_core
  DEBUG_POSTFIX "d")

# ImgProc module depends on core
add_library(opencv_imgproc)

target_sources(opencv_imgproc
  PRIVATE
    modules/imgproc/src/color.cpp
    modules/imgproc/src/filter.cpp
    modules/imgproc/src/smooth.cpp
    modules/imgproc/src/imgwarp.cpp
    modules/imgproc/src/resize.cpp
    modules/imgproc/src/thresh.cpp
    modules/imgproc/src/edge.cpp
    modules/imgproc/src/contours.cpp
    modules/imgproc/src/histogram.cpp)

target_include_directories(opencv_imgproc
  PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/modules/imgproc/include>
    $<INSTALL_INTERFACE:include/opencv4>)

target_link_libraries(opencv_imgproc
  PUBLIC opencv_core)

set_target_properties(opencv_imgproc PROPERTIES
  VERSION ${OPENCV_VERSION}
  OUTPUT_NAME opencv_imgproc
  DEBUG_POSTFIX "d")

# HighGUI module - platform-specific
if(NOT OPENCV_PLATFORM_IOS AND NOT OPENCV_PLATFORM_ANDROID)
  add_library(opencv_highgui)

  target_sources(opencv_highgui
    PRIVATE
      modules/highgui/src/window.cpp
      modules/highgui/src/roiwindow.cpp)

  # Platform-specific window backend
  if(OPENCV_PLATFORM_WINDOWS)
    target_sources(opencv_highgui PRIVATE modules/highgui/src/window_w32.cpp)
    target_link_libraries(opencv_highgui PRIVATE comctl32 gdi32 ole32 setupapi ws2_32 vfw32)
  elseif(OPENCV_PLATFORM_MACOS)
    target_sources(opencv_highgui PRIVATE modules/highgui/src/window_cocoa.mm)
    target_link_libraries(opencv_highgui PRIVATE "-framework Cocoa")
  elseif(OPENCV_PLATFORM_LINUX)
    if(HAVE_GTK)
      target_sources(opencv_highgui PRIVATE modules/highgui/src/window_gtk.cpp)
      if(HAVE_GTK3)
        target_link_libraries(opencv_highgui PRIVATE ${GTK3_LIBRARIES})
        target_include_directories(opencv_highgui PRIVATE ${GTK3_INCLUDE_DIRS})
      else()
        target_link_libraries(opencv_highgui PRIVATE ${GTK2_LIBRARIES})
        target_include_directories(opencv_highgui PRIVATE ${GTK2_INCLUDE_DIRS})
      endif()
    else()
      message(WARNING "No window backend available for HighGUI on Linux")
    endif()
  endif()

  target_include_directories(opencv_highgui
    PUBLIC
      $<BUILD_INTERFACE:${CMAKE_CURRENT_SOURCE_DIR}/modules/highgui/include>
      $<INSTALL_INTERFACE:include/opencv4>)

  target_link_libraries(opencv_highgui
    PUBLIC
      opencv_core
      opencv_imgproc)

  set_target_properties(opencv_highgui PROPERTIES
    VERSION ${OPENCV_VERSION}
    OUTPUT_NAME opencv_highgui
    DEBUG_POSTFIX "d")
endif()

# Installation
install(TARGETS opencv_core opencv_imgproc
  EXPORT OpenCVTargets
  RUNTIME DESTINATION bin
    COMPONENT runtime
  LIBRARY DESTINATION lib
    COMPONENT runtime
  ARCHIVE DESTINATION lib
    COMPONENT devel)

if(TARGET opencv_highgui)
  install(TARGETS opencv_highgui
    EXPORT OpenCVTargets
    RUNTIME DESTINATION bin COMPONENT runtime
    LIBRARY DESTINATION lib COMPONENT runtime
    ARCHIVE DESTINATION lib COMPONENT devel)
endif()

# Install headers
install(DIRECTORY modules/core/include/opencv2/
  DESTINATION include/opencv4/opencv2
  COMPONENT devel
  FILES_MATCHING PATTERN "*.h" PATTERN "*.hpp")

install(DIRECTORY modules/imgproc/include/opencv2/
  DESTINATION include/opencv4/opencv2
  COMPONENT devel
  FILES_MATCHING PATTERN "*.h" PATTERN "*.hpp")

# CMake config files
include(CMakePackageConfigHelpers)

configure_package_config_file(
  cmake/OpenCVConfig.cmake.in
  "${CMAKE_CURRENT_BINARY_DIR}/OpenCVConfig.cmake"
  INSTALL_DESTINATION lib/cmake/opencv4)

write_basic_package_version_file(
  "${CMAKE_CURRENT_BINARY_DIR}/OpenCVConfigVersion.cmake"
  VERSION ${OPENCV_VERSION}
  COMPATIBILITY AnyNewerVersion)

install(FILES
  "${CMAKE_CURRENT_BINARY_DIR}/OpenCVConfig.cmake"
  "${CMAKE_CURRENT_BINARY_DIR}/OpenCVConfigVersion.cmake"
  DESTINATION lib/cmake/opencv4
  COMPONENT devel)

install(EXPORT OpenCVTargets
  FILE OpenCVTargets.cmake
  NAMESPACE OpenCV::
  DESTINATION lib/cmake/opencv4
  COMPONENT devel)

# Summary of configuration
message(STATUS "")
message(STATUS "OpenCV configuration summary:")
message(STATUS "  Version: ${OPENCV_VERSION}")
message(STATUS "  Platform:")
message(STATUS "    Host: ${CMAKE_HOST_SYSTEM_NAME} ${CMAKE_HOST_SYSTEM_VERSION} ${CMAKE_HOST_SYSTEM_PROCESSOR}")
message(STATUS "    Target: ${CMAKE_SYSTEM_NAME} ${CMAKE_SYSTEM_VERSION} ${CMAKE_SYSTEM_PROCESSOR}")
message(STATUS "  C++ Compiler: ${CMAKE_CXX_COMPILER_ID} ${CMAKE_CXX_COMPILER_VERSION}")
message(STATUS "  Build:")
message(STATUS "    Shared libs: ${BUILD_SHARED_LIBS}")
if(X86_64)
  message(STATUS "    CPU baseline: SSE SSE2 SSE3")
  if(ENABLE_AVX2)
    message(STATUS "    CPU dispatch: AVX AVX2")
  elseif(ENABLE_AVX)
    message(STATUS "    CPU dispatch: AVX")
  endif()
endif()
message(STATUS "  Optional dependencies:")
message(STATUS "    CUDA: ${HAVE_CUDA}")
message(STATUS "    OpenCL: ${HAVE_OPENCL}")
message(STATUS "    TBB: ${HAVE_TBB}")
message(STATUS "    Eigen: ${HAVE_EIGEN}")
if(OPENCV_PLATFORM_LINUX)
  message(STATUS "    GTK: ${HAVE_GTK}")
endif()
message(STATUS "")
