cmake_minimum_required(VERSION 3.20)
project(mcml-core)

file(GLOB_RECURSE CPP_SOURCES "src/*.cpp")

add_library(mcml-core STATIC ${CPP_SOURCES})

target_include_directories(mcml-core
    PUBLIC
        ${CMAKE_CURRENT_SOURCE_DIR}/include
    PRIVATE
        ${CMAKE_CURRENT_SOURCE_DIR}/src
)

target_link_libraries(mcml-core PRIVATE
    mcml-async
    mcml-log
    Boost::asio
)

if(MSVC)
    target_compile_options(mcml-core PRIVATE "/std:c++latest" "/utf-8" "/bigobj")
    add_definitions(-D_WIN32_WINNT=0x0601)
endif()