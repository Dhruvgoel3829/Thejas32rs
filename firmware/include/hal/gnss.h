#ifndef HAL_GNSS_H
#define HAL_GNSS_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    int32_t lat_e7;
    int32_t lon_e7;
    int32_t alt_mm;
    int32_t vn_mm_s, ve_mm_s, vd_mm_s;
    uint8_t fix_type;
    uint8_t sats;
    uint16_t hdop_x100;
    time_us_t t_us;
} gnss_raw_t;

/* NMEA parser is portable - HAL feeds raw bytes to it */
typedef struct {
    uint8_t buf[256];
    uint16_t head, tail;
} nmea_rx_t;

bool hal_gnss_init(void);
bool hal_gnss_read(gnss_raw_t *out);

/* Feed NMEA byte to portable parser */
void nmea_parser_feed(nmea_rx_t *rx, uint8_t byte);
bool nmea_parser_parse(nmea_rx_t *rx, gnss_raw_t *out);

#ifdef __cplusplus
}
#endif

#endif /* HAL_GNSS_H */