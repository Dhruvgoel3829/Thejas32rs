#ifndef HAL_RC_H
#define HAL_RC_H

#include "types.h"

#ifdef __cplusplus
extern "C" {
#endif

#define RC_MAX_CHANNELS 16

typedef struct {
    uint16_t channels[RC_MAX_CHANNELS];
    bool link_ok;
    time_us_t last_frame_us;
} rc_raw_t;

/* RC protocol parsing is portable - HAL feeds raw bytes */
typedef struct {
    uint8_t buf[64];
    uint16_t head, tail;
} rc_rx_t;

bool hal_rc_init(void);
bool hal_rc_read(rc_raw_t *out);

/* Feed byte to portable SBUS/CRSF/PPM parser */
void rc_parser_feed(rc_rx_t *rx, uint8_t byte);
bool rc_parser_parse(rc_rx_t *rx, rc_raw_t *out);

#ifdef __cplusplus
}
#endif

#endif /* HAL_RC_H */