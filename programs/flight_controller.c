#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <arpa/inet.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <fcntl.h>
#include <errno.h>

#define IMU_PORT       9002
#define MOTOR_PORT     9003
#define SERVER_IP      "127.0.0.1"

#define MOTOR_MIN      0
#define MOTOR_IDLE     25
#define MOTOR_MAX      250

static inline int clamp(int val, int min, int max) {
    if (val < min) return min;
    if (val > max) return max;
    return val;
}

typedef struct {
    int kp;
    int ki;
    int kd;
    int integral;
    int prev_error;
} pid_ctrl_t;

int pid_update(pid_ctrl_t *pid, int target, int current) {
    int error = target - current;

    int p_out = (pid->kp * error) / 100;

    pid->integral += error;
    pid->integral = clamp(pid->integral, -500, 500);
    int i_out = (pid->ki * pid->integral) / 100;

    int derivative = error - pid->prev_error;
    pid->prev_error = error;
    int d_out = (pid->kd * derivative) / 100;

    return p_out + i_out + d_out;
}

void write_motors(int sock, struct sockaddr_in *addr, int throttle, int roll_corr, int pitch_corr, int yaw_corr) {
    if (throttle < MOTOR_IDLE) {
        float cmd[4] = {0, 0, 0, 0};
        sendto(sock, cmd, sizeof(cmd), 0, (struct sockaddr*)addr, sizeof(*addr));
        return;
    }

    int m1 = throttle - pitch_corr - roll_corr + yaw_corr;
    int m2 = throttle + pitch_corr - roll_corr - yaw_corr;
    int m3 = throttle - pitch_corr + roll_corr - yaw_corr;
    int m4 = throttle + pitch_corr + roll_corr + yaw_corr;

    float out1 = clamp(m1, MOTOR_IDLE, MOTOR_MAX) / 255.0f;
    float out2 = clamp(m2, MOTOR_IDLE, MOTOR_MAX) / 255.0f;
    float out3 = clamp(m3, MOTOR_IDLE, MOTOR_MAX) / 255.0f;
    float out4 = clamp(m4, MOTOR_IDLE, MOTOR_MAX) / 255.0f;

    float cmd[4] = {out1, out2, out3, out4};
    sendto(sock, cmd, sizeof(cmd), 0, (struct sockaddr*)addr, sizeof(*addr));

    printf("M1: %.2f | M2: %.2f | M3: %.2f | M4: %.2f\n", out1, out2, out3, out4);
}

int main(void) {
    int imu_sock = socket(AF_INET, SOCK_DGRAM, 0);
    int motor_sock = socket(AF_INET, SOCK_DGRAM, 0);
    if (imu_sock < 0 || motor_sock < 0) {
        perror("socket");
        return 1;
    }

    struct sockaddr_in imu_addr, motor_addr, from_addr;
    socklen_t addr_len = sizeof(from_addr);

    memset(&imu_addr, 0, sizeof(imu_addr));
    imu_addr.sin_family = AF_INET;
    imu_addr.sin_port = htons(IMU_PORT);
    imu_addr.sin_addr.s_addr = htonl(INADDR_ANY);

    memset(&motor_addr, 0, sizeof(motor_addr));
    motor_addr.sin_family = AF_INET;
    motor_addr.sin_port = htons(MOTOR_PORT);
    inet_pton(AF_INET, SERVER_IP, &motor_addr.sin_addr);

    if (bind(imu_sock, (struct sockaddr*)&imu_addr, sizeof(imu_addr)) < 0) {
        perror("bind");
        return 1;
    }

    struct timeval tv = { .tv_sec = 0, .tv_usec = 2000 };
    setsockopt(imu_sock, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));

    printf("====================================================\n");
    printf("   THEJAS32 Closed-Loop Flight Controller (UDP)     \n");
    printf("====================================================\n");

    pid_ctrl_t roll_pid  = { .kp = 120, .ki = 5, .kd = 40, .integral = 0, .prev_error = 0 };
    pid_ctrl_t pitch_pid = { .kp = 120, .ki = 5, .kd = 40, .integral = 0, .prev_error = 0 };

    int target_roll_rate  = 0;
    int target_pitch_rate = 0;

    int cycle = 0;
    double imu_pkt[7];
    float motor_cmd[4] = {0};

    while (1) {
        cycle++;

        ssize_t len = recvfrom(imu_sock, imu_pkt, sizeof(imu_pkt), 0, (struct sockaddr*)&from_addr, &addr_len);
        if (len == sizeof(imu_pkt)) {
            double sim_time = imu_pkt[0];
            double ax = imu_pkt[1], ay = imu_pkt[2], az = imu_pkt[3];
            double gx = imu_pkt[4], gy = imu_pkt[5], gz = imu_pkt[6];

            int current_roll_rate  = (int)(gx * 57.2958 * 16.4);
            int current_pitch_rate = (int)(gy * 57.2958 * 16.4);

            int roll_corr  = pid_update(&roll_pid, target_roll_rate, current_roll_rate);
            int pitch_corr = pid_update(&pitch_pid, target_pitch_rate, current_pitch_rate);

            write_motors(motor_sock, &motor_addr, 135, roll_corr, pitch_corr, 0);

            if ((cycle % 20) == 0) {
                printf("Loop #%d | Roll Rate: %d dps | Roll Corr: %d\n", cycle, current_roll_rate, roll_corr);
            }
        } else if (len < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
            perror("recvfrom");
        }

        usleep(2000);
    }

    close(imu_sock);
    close(motor_sock);
    return 0;
}