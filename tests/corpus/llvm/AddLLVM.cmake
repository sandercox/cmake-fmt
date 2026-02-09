# AddLLVM.cmake - LLVM build system utilities
# This module provides functions and macros used throughout LLVM's build system
# Demonstrates: function definitions, foreach loops, string manipulation, install commands

include(CMakeParseArguments)

# Create a custom LLVM library target
# Usage: llvm_add_library(name [STATIC|SHARED|MODULE] sources...)
function(llvm_add_library name)
  cmake_parse_arguments(ARG
    "STATIC;SHARED;MODULE;OBJECT;DISABLE_LLVM_LINK_LLVM_DYLIB"
    "OUTPUT_NAME;PLUGIN_TOOL"
    "ADDITIONAL_HEADERS;ADDITIONAL_HEADER_DIRS;DEPENDS;LINK_COMPONENTS;LINK_LIBS"
    ${ARGN})

  set(ALL_FILES ${ARG_UNPARSED_ARGUMENTS})

  # Determine library type
  if(ARG_SHARED)
    set(LIBTYPE SHARED)
  elseif(ARG_MODULE)
    set(LIBTYPE MODULE)
  elseif(ARG_OBJECT)
    set(LIBTYPE OBJECT)
  else()
    # Default to STATIC
    set(LIBTYPE STATIC)
  endif()

  # Handle additional headers for IDE integration
  if(ARG_ADDITIONAL_HEADERS)
    set_source_files_properties(${ARG_ADDITIONAL_HEADERS}
      PROPERTIES HEADER_FILE_ONLY ON)
    list(APPEND ALL_FILES ${ARG_ADDITIONAL_HEADERS})
  endif()

  if(ARG_ADDITIONAL_HEADER_DIRS)
    foreach(hdr_dir ${ARG_ADDITIONAL_HEADER_DIRS})
      file(GLOB hdr_files "${hdr_dir}/*.h" "${hdr_dir}/*.inc" "${hdr_dir}/*.def")
      if(hdr_files)
        set_source_files_properties(${hdr_files} PROPERTIES HEADER_FILE_ONLY ON)
        list(APPEND ALL_FILES ${hdr_files})
      endif()
    endforeach()
  endif()

  # Create the library target
  add_library(${name} ${LIBTYPE} ${ALL_FILES})

  # Set output name if specified
  if(ARG_OUTPUT_NAME)
    set_target_properties(${name} PROPERTIES OUTPUT_NAME ${ARG_OUTPUT_NAME})
  endif()

  # Add dependencies
  if(ARG_DEPENDS)
    add_dependencies(${name} ${ARG_DEPENDS})
  endif()

  # Link components (LLVM-specific)
  if(ARG_LINK_COMPONENTS)
    llvm_map_components_to_libnames(llvm_libs ${ARG_LINK_COMPONENTS})
    target_link_libraries(${name} PRIVATE ${llvm_libs})
  endif()

  # Link additional libraries
  if(ARG_LINK_LIBS)
    target_link_libraries(${name} PRIVATE ${ARG_LINK_LIBS})
  endif()

  # Set common properties
  set_target_properties(${name} PROPERTIES
    FOLDER "LLVM libraries"
    POSITION_INDEPENDENT_CODE ON)

  # Installation
  if(NOT ARG_MODULE AND NOT ARG_OBJECT)
    install(TARGETS ${name}
      EXPORT LLVMExports
      ARCHIVE DESTINATION lib${LLVM_LIBDIR_SUFFIX} COMPONENT ${name}
      LIBRARY DESTINATION lib${LLVM_LIBDIR_SUFFIX} COMPONENT ${name}
      RUNTIME DESTINATION bin COMPONENT ${name})
  endif()
endfunction()

# Add an LLVM executable
function(llvm_add_executable name)
  cmake_parse_arguments(ARG
    "DISABLE_LLVM_LINK_LLVM_DYLIB;IGNORE_EXTERNALIZE_DEBUGINFO;NO_INSTALL_RPATH;SUPPORT_PLUGINS"
    "ENTITLEMENTS;BUNDLE_PATH"
    "DEPENDS;LINK_COMPONENTS;LINK_LIBS"
    ${ARGN})

  set(ALL_FILES ${ARG_UNPARSED_ARGUMENTS})

  add_executable(${name} ${ALL_FILES})

  # Add dependencies
  if(ARG_DEPENDS)
    add_dependencies(${name} ${ARG_DEPENDS})
  endif()

  # Link LLVM components
  if(ARG_LINK_COMPONENTS)
    llvm_map_components_to_libnames(llvm_libs ${ARG_LINK_COMPONENTS})
    target_link_libraries(${name} PRIVATE ${llvm_libs})
  endif()

  # Link additional libraries
  if(ARG_LINK_LIBS)
    target_link_libraries(${name} PRIVATE ${ARG_LINK_LIBS})
  endif()

  # Set RPATH for installed binaries
  if(NOT ARG_NO_INSTALL_RPATH)
    if(APPLE)
      set_target_properties(${name} PROPERTIES
        INSTALL_RPATH "@executable_path/../lib")
    elseif(UNIX)
      set_target_properties(${name} PROPERTIES
        INSTALL_RPATH "$ORIGIN/../lib${LLVM_LIBDIR_SUFFIX}")
    endif()
  endif()

  # Support for plugins
  if(ARG_SUPPORT_PLUGINS)
    if(WIN32)
      target_link_options(${name} PRIVATE
        /EXPORT:LLVMGetPassPluginInfo)
    elseif(APPLE)
      set_target_properties(${name} PROPERTIES
        ENABLE_EXPORTS ON)
    else()
      target_link_options(${name} PRIVATE
        -Wl,--export-dynamic)
    endif()
  endif()

  set_target_properties(${name} PROPERTIES FOLDER "LLVM executables")
endfunction()

# Map LLVM component names to actual library names
# This is a simplified version of LLVM's actual implementation
function(llvm_map_components_to_libnames out_libs)
  set(link_libs)

  foreach(comp ${ARGN})
    # Convert component names to library names
    # Example: "core" -> "LLVMCore", "support" -> "LLVMSupport"
    string(SUBSTRING ${comp} 0 1 first_letter)
    string(TOUPPER ${first_letter} first_letter_upper)
    string(SUBSTRING ${comp} 1 -1 rest)
    set(lib_name "LLVM${first_letter_upper}${rest}")

    # Check if target exists
    if(TARGET ${lib_name})
      list(APPEND link_libs ${lib_name})
    else()
      message(WARNING "LLVM component '${comp}' not found, skipping")
    endif()
  endforeach()

  set(${out_libs} ${link_libs} PARENT_SCOPE)
endfunction()

# Add a new LLVM tool subdirectory
macro(llvm_add_tool_subdirectory name)
  add_subdirectory(${name})
  set_target_properties(${name} PROPERTIES FOLDER "LLVM tools")
endmacro()

# Install headers for a component
function(llvm_install_component_headers component)
  cmake_parse_arguments(ARG
    ""
    "DESTINATION"
    "HEADERS;PATTERNS"
    ${ARGN})

  if(NOT ARG_DESTINATION)
    set(ARG_DESTINATION "include/llvm/${component}")
  endif()

  # Install specified headers
  if(ARG_HEADERS)
    foreach(hdr ${ARG_HEADERS})
      get_filename_component(hdr_dir ${hdr} DIRECTORY)
      install(FILES ${hdr}
        DESTINATION ${ARG_DESTINATION}
        COMPONENT llvm-headers)
    endforeach()
  endif()

  # Install headers matching patterns
  if(ARG_PATTERNS)
    foreach(pattern ${ARG_PATTERNS})
      file(GLOB pattern_files ${pattern})
      if(pattern_files)
        install(FILES ${pattern_files}
          DESTINATION ${ARG_DESTINATION}
          COMPONENT llvm-headers)
      endif()
    endforeach()
  endif()
endfunction()

# Tablegen support (code generation from .td files)
function(llvm_tablegen output_file td_file)
  cmake_parse_arguments(ARG
    ""
    ""
    "DEPENDS;EXTRA_INCLUDES"
    ${ARGN})

  set(LLVM_TABLEGEN_EXE llvm-tblgen)
  set(LLVM_TABLEGEN_FLAGS "")

  # Add extra include directories
  foreach(inc_dir ${ARG_EXTRA_INCLUDES})
    list(APPEND LLVM_TABLEGEN_FLAGS "-I" "${inc_dir}")
  endforeach()

  # Generate custom command
  add_custom_command(
    OUTPUT ${output_file}
    COMMAND ${LLVM_TABLEGEN_EXE} ${LLVM_TABLEGEN_FLAGS} -o ${output_file} ${td_file}
    DEPENDS ${LLVM_TABLEGEN_EXE} ${td_file} ${ARG_DEPENDS}
    COMMENT "Building ${output_file} from ${td_file}"
    VERBATIM)

  # Make output available
  set_source_files_properties(${output_file} PROPERTIES GENERATED ON)
endfunction()

# Add a unit test
function(llvm_add_unittest test_suite test_name)
  set(test_target ${test_suite}Tests.${test_name})

  add_executable(${test_target} ${ARGN})

  target_link_libraries(${test_target}
    PRIVATE
      gtest
      gtest_main
      LLVMSupport)

  add_test(NAME ${test_target}
    COMMAND ${test_target})

  set_target_properties(${test_target} PROPERTIES
    FOLDER "LLVM tests/${test_suite}")
endfunction()

# Process subdirectories for out-of-tree builds
macro(llvm_process_sources)
  cmake_parse_arguments(ARG
    ""
    ""
    "ADDITIONAL_HEADERS;ADDITIONAL_HEADER_DIRS"
    ${ARGN})

  # This would normally do complex processing
  # Simplified for demonstration
  foreach(src ${ARG_UNPARSED_ARGUMENTS})
    get_filename_component(src_ext ${src} EXT)
    if(src_ext STREQUAL ".h" OR src_ext STREQUAL ".hpp")
      set_source_files_properties(${src} PROPERTIES HEADER_FILE_ONLY ON)
    endif()
  endforeach()
endmacro()
