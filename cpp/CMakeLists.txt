cmake_minimum_required(VERSION 3.15)
project(rnim_ffi_cpp VERSION 0.1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# The library that wraps LLVM or external optimizer integration
add_library(rnim_ffi_cpp STATIC
    src/lib.cpp
    src/llvm_bridge.cpp
)

target_include_directories(rnim_ffi_cpp PUBLIC
    ${CMAKE_CURRENT_SOURCE_DIR}/include
)

target_compile_options(rnim_ffi_cpp PRIVATE
    -Wall
    -Wextra
)

# Install headers
install(DIRECTORY include/ DESTINATION include/rnim_ffi_cpp)