cmake_minimum_required(VERSION 3.10)
project(thejas_gazebo_sitl)

find_package(gazebo REQUIRED)
include_directories(${GAZEBO_INCLUDE_DIRS})
link_directories(${GAZEBO_LIBRARY_DIRS})

set(CMAKE_CXX_FLAGS "${CMAKE_CXX_FLAGS} ${GAZEBO_CXX_FLAGS} -std=c++17 -DTBB_PREVIEW_TASK=1")

add_library(thejas_sitl SHARED gazebo_thejas_sitl.cpp)
target_link_libraries(thejas_sitl ${GAZEBO_LIBRARIES})