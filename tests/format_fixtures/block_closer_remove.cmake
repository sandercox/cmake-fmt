# Remove mode input: legacy closers with arguments
if(WIN32)
  set(PLATFORM windows)
else(WIN32)
  set(PLATFORM unix)
endif(WIN32)

foreach(item ${LIST})
  message(${item})
endforeach(item)

function(my_func ARG1 ARG2)
  message(${ARG1})
endfunction(my_func)

macro(my_macro X Y)
  set(${X} ${Y})
endmacro(my_macro)

while(${COUNTER} LESS 10)
  math(EXPR COUNTER "${COUNTER} + 1")
endwhile(${COUNTER} LESS 10)

# Already empty closers stay empty
if(UNIX)
  set(A b)
endif()

# Nested blocks with legacy closers
if(WIN32)
  if(MSVC)
    set(COMPILER msvc)
  endif(MSVC)
  set(B c)
endif(WIN32)

# elseif with arguments
if(WIN32)
  set(A windows)
elseif(APPLE)
  set(A apple)
else(WIN32)
  set(A other)
endif(WIN32)
