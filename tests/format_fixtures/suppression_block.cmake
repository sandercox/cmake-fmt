cmake_minimum_required(VERSION 3.14)

set(FORMATTED_VAR value)

# cmake-fmt: off
set(  UGLY_VAR    value1   value2  )
message(   "unformatted"   )
# cmake-fmt: on

set(BACK_TO_NORMAL value)
