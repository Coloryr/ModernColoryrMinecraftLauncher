cmake_minimum_required(VERSION 3.21)

file(GLOB_RECURSE CPP_SOURCES "src/*.cpp")
file(GLOB_RECURSE UI_SOURCES "ui/*.slint")

add_executable(mcml ${CPP_SOURCES})

target_include_directories(mcml PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

slint_target_sources(mcml ${UI_SOURCES})

target_link_libraries(mcml PRIVATE 
    Slint::Slint
    mcml-core
    mcml-async)

set_property(TARGET mcml APPEND PROPERTY WIN32_EXECUTABLE TRUE)
set_property(TARGET mcml PROPERTY SLINT_EMBED_RESOURCES embed-files)

if(MSVC)
    target_compile_options(mcml PRIVATE "/std:c++latest" "/utf-8")
    add_definitions(-D_WIN32_WINNT=0x0601)
endif()

if (WIN32)
    add_custom_command(TARGET mcml POST_BUILD COMMAND ${CMAKE_COMMAND} -E copy $<TARGET_RUNTIME_DLLS:mcml> $<TARGET_FILE_DIR:mcml> COMMAND_EXPAND_LISTS)
endif()