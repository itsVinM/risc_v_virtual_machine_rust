cmake_minimum_required(VERSION 3.16)
project(rv64vm-tools LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

add_library(rv64elf STATIC lib/elf.cpp)
target_include_directories(rv64elf PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/lib)
target_compile_options(rv64elf PRIVATE -Wall -Wextra -Werror)

add_executable(elfdump elfdump/main.cpp)
target_link_libraries(elfdump PRIVATE rv64elf)

enable_testing()

add_executable(elf_tests tests/elf_tests.cpp)
target_link_libraries(elf_tests PRIVATE rv64elf)
add_test(NAME elf_tests COMMAND elf_tests)
