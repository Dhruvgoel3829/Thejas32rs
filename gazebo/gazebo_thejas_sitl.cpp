#include <gazebo/physics/Model.hh>
#include <gazebo/physics/Joint.hh>
#include <gazebo/physics/World.hh>
#include <gazebo/sensors/ImuSensor.hh>
#include <gazebo/sensors/Sensor.hh>
#include <gazebo/sensors/SensorManager.hh>
#include <gazebo/common/Plugin.hh>
#include <gazebo/common/Events.hh>
#include <gazebo/common/Time.hh>
#include <ignition/math/Vector3.hh>
#include <arpa/inet.h>
#include <sys/socket.h>
#include <fcntl.h>
#include <thread>
#include <mutex>

namespace gazebo {

class ThejasSitlPlugin : public ModelPlugin {
private:
    physics::ModelPtr model;
    physics::JointPtr motors[4];
    sensors::ImuSensorPtr imu_sensor;
    event::ConnectionPtr update_connection;

    int sock_fd;
    struct sockaddr_in bridge_addr;
    struct sockaddr_in bind_addr;

    double motor_cmd[4] = {0.0, 0.0, 0.0, 0.0};
    std::mutex cmd_mutex;
    bool running = true;
    std::thread rx_thread;

public:
    ThejasSitlPlugin() : sock_fd(-1) {}

    ~ThejasSitlPlugin() {
        running = false;
        if (sock_fd >= 0) close(sock_fd);
        if (rx_thread.joinable()) rx_thread.join();
    }

    void Load(physics::ModelPtr _model, sdf::ElementPtr _sdf) override {
        model = _model;

        motors[0] = model->GetJoint("rotor_0_joint");
        motors[1] = model->GetJoint("rotor_1_joint");
        motors[2] = model->GetJoint("rotor_2_joint");
        motors[3] = model->GetJoint("rotor_3_joint");

        sensors::SensorPtr s = sensors::SensorManager::Instance()->GetSensor("iris::base_link::imu_sensor");
        imu_sensor = std::dynamic_pointer_cast<sensors::ImuSensor>(s);

        sock_fd = socket(AF_INET, SOCK_DGRAM, 0);

        memset(&bind_addr, 0, sizeof(bind_addr));
        bind_addr.sin_family = AF_INET;
        bind_addr.sin_port = htons(9003);
        bind_addr.sin_addr.s_addr = htonl(INADDR_ANY);
        bind(sock_fd, (struct sockaddr*)&bind_addr, sizeof(bind_addr));

        memset(&bridge_addr, 0, sizeof(bridge_addr));
        bridge_addr.sin_family = AF_INET;
        bridge_addr.sin_port = htons(9002);
        inet_pton(AF_INET, "127.0.0.1", &bridge_addr.sin_addr);

        rx_thread = std::thread(&ThejasSitlPlugin::ReceiveMotors, this);

        update_connection = event::Events::ConnectWorldUpdateBegin(
            std::bind(&ThejasSitlPlugin::OnUpdate, this)
        );
    }

    void ReceiveMotors() {
        float buf[4];
        while (running) {
            ssize_t len = recvfrom(sock_fd, buf, sizeof(buf), 0, nullptr, nullptr);
            if (len == sizeof(buf)) {
                std::lock_guard<std::mutex> lock(cmd_mutex);
                for (int i = 0; i < 4; i++) {
                    motor_cmd[i] = buf[i];
                }
            }
        }
    }

    void OnUpdate() {
        common::Time cur_time = model->GetWorld()->SimTime();

        ignition::math::Vector3d lin_acc(0, 0, 9.81);
        ignition::math::Vector3d ang_vel(0, 0, 0);

        if (imu_sensor) {
            lin_acc = imu_sensor->LinearAcceleration();
            ang_vel = imu_sensor->AngularVelocity();
        }

        double imu_pkt[7] = {
            cur_time.Double(),
            lin_acc.X(), lin_acc.Y(), lin_acc.Z(),
            ang_vel.X(), ang_vel.Y(), ang_vel.Z()
        };

        sendto(sock_fd, imu_pkt, sizeof(imu_pkt), 0,
               (struct sockaddr*)&bridge_addr, sizeof(bridge_addr));

        const double MAX_VEL = 1500.0;
        std::lock_guard<std::mutex> lock(cmd_mutex);
        for (int i = 0; i < 4; i++) {
            if (motors[i]) {
                motors[i]->SetVelocity(0, motor_cmd[i] * MAX_VEL);
            }
        }
    }
};

GZ_REGISTER_MODEL_PLUGIN(ThejasSitlPlugin)

}