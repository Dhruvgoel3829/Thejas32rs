cmake_minimum_required(VERSION 3.10)
project(thejas_gazebo_sitl)

find_package(gazebo REQUIRED)
include_directories(${GAZEBO_INCLUDE_DIRS})
link_directories(${GAZEBO_LIBRARY_DIRS})

# Add TBB legacy / preview flags & enforce C++17
add_compile_definitions(
    TBB_PREVIEW_TASK_ARENA=1
    TBB_SUPPRESS_DEPRECATED_MESSAGES=1
)

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${GAZEBO_CXX_FLAGS} -std=c++17")
set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} -Wno-error=template-body")

add_library(thejas_sitl SHARED gazebo_thejas_sitl.cpp)
target_link_libraries(thejas_sitl ${GAZEBO_LIBRARIES})