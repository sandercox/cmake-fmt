# OpenCVModule.cmake - Module management utilities for OpenCV
# Demonstrates: macro/function definitions, foreach with RANGE, file(GLOB), string(REGEX), configure_file

include(CMakeParseArguments)

# Global list of OpenCV modules
set(OPENCV_MODULES_BUILD "" CACHE INTERNAL "List of OpenCV modules included in build")
set(OPENCV_MODULES_DISABLED_USER "" CACHE INTERNAL "List of OpenCV modules explicitly disabled by user")
set(OPENCV_MODULES_DISABLED_AUTO "" CACHE INTERNAL "List of OpenCV modules automatically disabled due to dependencies")

# Define an OpenCV module
# Usage: ocv_define_module(module_name [dependencies...] [OPTIONAL optional_deps...] [WRAP python java])
macro(ocv_define_module module_name)
  cmake_parse_arguments(OCV_MODULE
    ""
    ""
    "OPTIONAL;WRAP"
    ${ARGN})

  set(the_module opencv_${module_name})
  set(the_module_name ${module_name})

  # Store module dependencies
  set(OPENCV_MODULE_${the_module}_DEPS ${OCV_MODULE_UNPARSED_ARGUMENTS})
  set(OPENCV_MODULE_${the_module}_OPTIONAL_DEPS ${OCV_MODULE_OPTIONAL})
  set(OPENCV_MODULE_${the_module}_WRAPPERS ${OCV_MODULE_WRAP})

  # Check if this module should be built
  string(TOUPPER "BUILD_${the_module}" module_build_var)
  option(${module_build_var} "Build ${the_module} module" ON)

  if(${module_build_var})
    # Check required dependencies
    set(dependencies_ok TRUE)
    foreach(dep ${OPENCV_MODULE_${the_module}_DEPS})
      if(NOT TARGET opencv_${dep})
        set(dependencies_ok FALSE)
        list(APPEND OPENCV_MODULES_DISABLED_AUTO ${the_module})
        message(STATUS "Module ${the_module} disabled: missing dependency ${dep}")
        break()
      endif()
    endforeach()

    if(dependencies_ok)
      # Add to build list
      list(APPEND OPENCV_MODULES_BUILD ${the_module})
      set(OPENCV_MODULES_BUILD ${OPENCV_MODULES_BUILD} CACHE INTERNAL "")

      # Create the module target
      add_library(${the_module})

      # Set module properties
      set_target_properties(${the_module} PROPERTIES
        OUTPUT_NAME ${the_module}
        VERSION ${OPENCV_VERSION}
        SOVERSION ${OPENCV_VERSION_MAJOR}
        FOLDER "modules")

      # Link dependencies
      foreach(dep ${OPENCV_MODULE_${the_module}_DEPS})
        target_link_libraries(${the_module} PUBLIC opencv_${dep})
      endforeach()

      # Link optional dependencies if available
      foreach(opt_dep ${OPENCV_MODULE_${the_module}_OPTIONAL_DEPS})
        if(TARGET opencv_${opt_dep})
          target_link_libraries(${the_module} PRIVATE opencv_${opt_dep})
        endif()
      endforeach()
    endif()
  else()
    list(APPEND OPENCV_MODULES_DISABLED_USER ${the_module})
  endif()
endmacro()

# Add sources to an OpenCV module
# Usage: ocv_add_module_sources(module_name [sources...])
function(ocv_add_module_sources module_name)
  set(the_module opencv_${module_name})

  if(NOT TARGET ${the_module})
    message(FATAL_ERROR "Module ${the_module} does not exist")
  endif()

  # Collect sources
  set(module_sources)
  foreach(src ${ARGN})
    if(IS_ABSOLUTE ${src})
      list(APPEND module_sources ${src})
    else()
      list(APPEND module_sources "${CMAKE_CURRENT_SOURCE_DIR}/${src}")
    endif()
  endforeach()

  target_sources(${the_module} PRIVATE ${module_sources})
endfunction()

# Glob sources for a module
# Usage: ocv_glob_module_sources(module_name [PATTERNS patterns...])
function(ocv_glob_module_sources module_name)
  cmake_parse_arguments(OCV_GLOB
    ""
    ""
    "PATTERNS"
    ${ARGN})

  if(NOT OCV_GLOB_PATTERNS)
    set(OCV_GLOB_PATTERNS "src/*.cpp" "src/*.c" "include/*.hpp" "include/*.h")
  endif()

  set(the_module opencv_${module_name})
  set(module_path "${CMAKE_CURRENT_SOURCE_DIR}/modules/${module_name}")

  # Glob all files
  set(all_files)
  foreach(pattern ${OCV_GLOB_PATTERNS})
    file(GLOB pattern_files "${module_path}/${pattern}")
    list(APPEND all_files ${pattern_files})
  endforeach()

  # Separate sources and headers
  set(sources)
  set(headers)
  foreach(file ${all_files})
    get_filename_component(file_ext ${file} EXT)
    if(file_ext MATCHES "\\.(cpp|c|cc|cxx)$")
      list(APPEND sources ${file})
    elseif(file_ext MATCHES "\\.(h|hpp|hxx)$")
      list(APPEND headers ${file})
    endif()
  endforeach()

  if(sources)
    target_sources(${the_module} PRIVATE ${sources})
  endif()

  if(headers)
    set_source_files_properties(${headers} PROPERTIES HEADER_FILE_ONLY ON)
    target_sources(${the_module} PRIVATE ${headers})
  endif()
endfunction()

# Set include directories for a module
# Usage: ocv_module_include_directories(module_name [PUBLIC|PRIVATE|INTERFACE] dirs...)
function(ocv_module_include_directories module_name)
  set(the_module opencv_${module_name})

  if(NOT TARGET ${the_module})
    message(FATAL_ERROR "Module ${the_module} does not exist")
  endif()

  # Parse visibility
  set(visibility PUBLIC)
  if(${ARGV1} MATCHES "PUBLIC|PRIVATE|INTERFACE")
    set(visibility ${ARGV1})
    list(REMOVE_AT ARGN 0)
  endif()

  target_include_directories(${the_module} ${visibility} ${ARGN})
endfunction()

# Generate module configuration header
# Usage: ocv_generate_module_config(module_name)
function(ocv_generate_module_config module_name)
  set(the_module opencv_${module_name})
  set(config_file "${CMAKE_CURRENT_BINARY_DIR}/${module_name}_config.h")

  # Create config content
  string(TOUPPER ${module_name} module_name_upper)
  set(config_content "// Auto-generated configuration for ${the_module}\n")
  string(APPEND config_content "#ifndef OPENCV_${module_name_upper}_CONFIG_H\n")
  string(APPEND config_content "#define OPENCV_${module_name_upper}_CONFIG_H\n\n")

  # Add defines for enabled features
  get_target_property(compile_defs ${the_module} COMPILE_DEFINITIONS)
  if(compile_defs)
    foreach(def ${compile_defs})
      string(APPEND config_content "#define ${def}\n")
    endforeach()
  endif()

  string(APPEND config_content "\n#endif // OPENCV_${module_name_upper}_CONFIG_H\n")

  # Write file
  file(WRITE ${config_file} ${config_content})

  target_include_directories(${the_module} PUBLIC
    $<BUILD_INTERFACE:${CMAKE_CURRENT_BINARY_DIR}>)
endfunction()

# Install module files
# Usage: ocv_install_module(module_name [HEADERS header_dir])
function(ocv_install_module module_name)
  cmake_parse_arguments(OCV_INSTALL
    ""
    "HEADERS"
    ""
    ${ARGN})

  set(the_module opencv_${module_name})

  # Install the library
  install(TARGETS ${the_module}
    EXPORT OpenCVTargets
    RUNTIME DESTINATION bin COMPONENT runtime
    LIBRARY DESTINATION lib COMPONENT runtime
    ARCHIVE DESTINATION lib COMPONENT devel)

  # Install headers if specified
  if(OCV_INSTALL_HEADERS)
    set(header_dir "${CMAKE_CURRENT_SOURCE_DIR}/modules/${module_name}/${OCV_INSTALL_HEADERS}")
    if(EXISTS ${header_dir})
      install(DIRECTORY ${header_dir}/
        DESTINATION include/opencv4/opencv2/${module_name}
        COMPONENT devel
        FILES_MATCHING
          PATTERN "*.h"
          PATTERN "*.hpp"
          PATTERN "*.inl.hpp")
    endif()
  endif()
endfunction()

# Create a module test executable
# Usage: ocv_add_test(module_name test_name [sources...])
function(ocv_add_test module_name test_name)
  set(the_module opencv_${module_name})
  set(the_target ${the_module}_test_${test_name})

  add_executable(${the_target} ${ARGN})

  target_link_libraries(${the_target}
    PRIVATE
      ${the_module}
      opencv_ts  # OpenCV test support library
      gtest
      gtest_main)

  set_target_properties(${the_target} PROPERTIES
    FOLDER "tests/${module_name}")

  add_test(NAME ${the_target} COMMAND ${the_target})
endfunction()

# Create a performance test executable
# Usage: ocv_add_perf_test(module_name test_name [sources...])
function(ocv_add_perf_test module_name test_name)
  set(the_module opencv_${module_name})
  set(the_target ${the_module}_perf_${test_name})

  add_executable(${the_target} ${ARGN})

  target_link_libraries(${the_target}
    PRIVATE
      ${the_module}
      opencv_ts)

  set_target_properties(${the_target} PROPERTIES
    FOLDER "perf_tests/${module_name}")
endfunction()

# Generate version information for a module
# Usage: ocv_generate_version_info(module_name)
macro(ocv_generate_version_info module_name)
  set(the_module opencv_${module_name})
  set(version_file "${CMAKE_CURRENT_BINARY_DIR}/${module_name}_version.cpp")

  file(WRITE ${version_file} "
// Auto-generated version info for ${the_module}
#include <string>

namespace cv {
namespace ${module_name} {

const char* getVersionString() {
  return \"${OPENCV_VERSION}\";
}

int getVersionMajor() {
  return ${OPENCV_VERSION_MAJOR};
}

int getVersionMinor() {
  return ${OPENCV_VERSION_MINOR};
}

int getVersionPatch() {
  return ${OPENCV_VERSION_PATCH};
}

} // namespace ${module_name}
} // namespace cv
")

  target_sources(${the_module} PRIVATE ${version_file})
endmacro()

# Check platform-specific features
# Usage: ocv_check_platform_features(module_name [features...])
function(ocv_check_platform_features module_name)
  set(the_module opencv_${module_name})

  foreach(feature ${ARGN})
    # Check for common features
    if(feature STREQUAL "SSE2")
      include(CheckCXXSourceCompiles)
      check_cxx_source_compiles("
        #include <emmintrin.h>
        int main() { __m128d r = _mm_set_sd(0.0); return 0; }
      " HAVE_SSE2)

      if(HAVE_SSE2)
        target_compile_definitions(${the_module} PRIVATE CV_CPU_HAS_SUPPORT_SSE2=1)
      endif()
    elseif(feature STREQUAL "AVX2")
      check_cxx_source_compiles("
        #include <immintrin.h>
        int main() { __m256d r = _mm256_set1_pd(0.0); return 0; }
      " HAVE_AVX2)

      if(HAVE_AVX2)
        target_compile_definitions(${the_module} PRIVATE CV_CPU_HAS_SUPPORT_AVX2=1)
      endif()
    elseif(feature STREQUAL "NEON")
      check_cxx_source_compiles("
        #include <arm_neon.h>
        int main() { float32x4_t r = vdupq_n_f32(0.0f); return 0; }
      " HAVE_NEON)

      if(HAVE_NEON)
        target_compile_definitions(${the_module} PRIVATE CV_CPU_HAS_SUPPORT_NEON=1)
      endif()
    endif()
  endforeach()
endfunction()

# Print module build status summary
function(ocv_print_module_summary)
  message(STATUS "")
  message(STATUS "OpenCV modules:")
  message(STATUS "  To be built: ${OPENCV_MODULES_BUILD}")

  if(OPENCV_MODULES_DISABLED_USER)
    message(STATUS "  Disabled by user: ${OPENCV_MODULES_DISABLED_USER}")
  endif()

  if(OPENCV_MODULES_DISABLED_AUTO)
    message(STATUS "  Disabled automatically: ${OPENCV_MODULES_DISABLED_AUTO}")
  endif()

  message(STATUS "")
endfunction()
