cmake_minimum_required(VERSION 3.20)
project(mcml-log)

file(GLOB_RECURSE CPP_SOURCES "src/*.cpp")

add_library(mcml-log STATIC ${CPP_SOURCES})

target_include_directories(mcml-log
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_link_libraries(mcml-log PRIVATE
    mcml-async
    Boost::lockfree
)

if(MSVC)
    target_compile_options(mcml-log PRIVATE "/std:c++latest" "/utf-8")
    add_definitions(-D_WIN32_WINNT=0x0601)
endif()