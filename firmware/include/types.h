#ifndef TYPES_H
#define TYPES_H

/*
 * VEGA Pilot Firmware Common Types
 * Pure C11, no hardware dependencies.
 */

#include <stdint.h>
#include <stdbool.h>
#include <stddef.h>

/* Status codes used throughout HAL and modules */
typedef enum {
    STATUS_OK = 0,
    STATUS_ERR = -1,
    STATUS_ERR_INVALID_ARG = -2,
    STATUS_ERR_NOT_READY = -3,
    STATUS_ERR_BUSY = -4,
    STATUS_ERR_TIMEOUT = -5,
    STATUS_ERR_NO_MEM = -6,
    STATUS_ERR_UNSUPPORTED = -7,
    STATUS_ERR_CRC = -8,
} status_t;

/* Time types */
typedef uint64_t time_us_t;
typedef uint32_t time_ms_t;
typedef uint64_t cycles_t;

/* Fixed-point types for parameters without FPU */
typedef int32_t q16_16_t;  /* 16.16 fixed point */
typedef int32_t q8_24_t;   /* 8.24 fixed point */

/* Euler angles in radians (float) */
typedef struct {
    float roll;
    float pitch;
    float yaw;
} euler_t;

/* Angular rates in rad/s (float) */
typedef struct {
    float x;
    float y;
    float z;
} rate_t;

/* 3D vector (float) */
typedef struct {
    float x;
    float y;
    float z;
} vec3_t;

/* Home position */
typedef struct {
    int32_t lat_e7;
    int32_t lon_e7;
    float alt_m;
    bool valid;
} home_t;

/* Mission item (subset of MAVLink MISSION_ITEM_INT) */
typedef struct {
    uint16_t seq;
    uint8_t frame;
    uint16_t command;
    uint8_t current;
    uint8_t autocontinue;
    float param1, param2, param3, param4;
    int32_t lat_e7;
    int32_t lon_e7;
    float alt_m;
    uint8_t mission_type;
} mission_item_t;

/* Parameter entry */
typedef struct {
    char name[16];
    float value;
    float min;
    float max;
    float def;
} param_entry_t;

/* MAVLink message buffer */
typedef struct {
    uint8_t buf[280];  /* Max MAVLink v2 frame size */
    uint16_t len;
    uint8_t sysid;
    uint8_t compid;
} mavlink_msg_t;

#endif /* TYPES_H */