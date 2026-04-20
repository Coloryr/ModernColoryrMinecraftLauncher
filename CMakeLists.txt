cmake_minimum_required(VERSION 3.21)
project(mcml)

set(CMAKE_CXX_STANDARD 23)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# slint
set(SLINT_FEATURE_BACKEND_QT OFF)
set(SLINT_FEATURE_BACKEND_WINIT ON)
set(SLINT_FEATURE_RENDERER_SKIA ON)

set(VCPKG_ROOT "${CMAKE_CURRENT_SOURCE_DIR}/vcpkg")
set(VCPKG_BOOTSTRAP_SCRIPT "")
set(VCPKG_EXECUTABLE "${VCPKG_ROOT}/vcpkg")

if(WIN32)
    set(VCPKG_BOOTSTRAP_SCRIPT "${VCPKG_ROOT}/bootstrap-vcpkg.bat")
    set(VCPKG_EXECUTABLE "${VCPKG_ROOT}/vcpkg.exe")
else()
    set(VCPKG_BOOTSTRAP_SCRIPT "${VCPKG_ROOT}/bootstrap-vcpkg.sh")
    set(VCPKG_EXECUTABLE "${VCPKG_ROOT}/vcpkg")
endif()

if(NOT EXISTS "${VCPKG_EXECUTABLE}")
    message(STATUS "vcpkg not found. Cloning and bootstrapping...")
    
    if(NOT EXISTS "${VCPKG_ROOT}/.git")
        execute_process(
            COMMAND git clone https://github.com/microsoft/vcpkg.git "${VCPKG_ROOT}"
            RESULT_VARIABLE GIT_CLONE_RESULT
        )
        if(NOT GIT_CLONE_RESULT EQUAL 0)
            message(FATAL_ERROR "Failed to clone vcpkg repository")
        endif()
    endif()
    
    execute_process(
        COMMAND "${VCPKG_BOOTSTRAP_SCRIPT}"
        WORKING_DIRECTORY "${VCPKG_ROOT}"
        RESULT_VARIABLE BOOTSTRAP_RESULT
    )
    if(NOT BOOTSTRAP_RESULT EQUAL 0)
        message(FATAL_ERROR "Failed to bootstrap vcpkg")
    endif()
endif()

# message(STATUS "vcpkg found at: ${VCPKG_EXECUTABLE}")
# set(CMAKE_TOOLCHAIN_FILE "${CMAKE_CURRENT_SOURCE_DIR}/vcpkg/scripts/buildsystems/vcpkg.cmake" 
#     CACHE STRING "vcpkg toolchain file")

include("${CMAKE_CURRENT_SOURCE_DIR}/vcpkg/scripts/buildsystems/vcpkg.cmake" )

# message(STATUS "Installing vcpkg dependencies from manifest...")
# execute_process(
#     COMMAND "${VCPKG_EXECUTABLE}" install
#     WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
#     RESULT_VARIABLE VCPKG_INSTALL_RESULT
# )
# if(NOT VCPKG_INSTALL_RESULT EQUAL 0)
#     message(FATAL_ERROR "Failed to install vcpkg dependencies")
# endif()

find_package(Boost REQUIRED COMPONENTS asio system beast lockfree)

include(FetchContent)
FetchContent_Declare(
    Slint
    GIT_REPOSITORY https://github.com/slint-ui/slint.git
    GIT_TAG release/1
    SOURCE_SUBDIR api/cpp
)

FetchContent_MakeAvailable(Slint)

add_subdirectory(mcml-log)
add_subdirectory(mcml-async)
add_subdirectory(mcml-core)
add_subdirectory(mcml)
