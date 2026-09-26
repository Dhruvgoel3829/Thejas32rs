#include <gazebo/physics/Model.hh>
#include <gazebo/physics/Joint.hh>
#include <gazebo/physics/World.hh>
#include <gazebo/physics/Link.hh>
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
    physics::LinkPtr base_link;
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

    // Physical constants
    const double MASS = 1.5;           // kg (from world file)
    const double GRAVITY = 9.81;       // m/s^2
    const double HOVER_THRUST = MASS * GRAVITY;  // 14.7N
    const double MAX_THRUST_PER_MOTOR = 5.0;     // 20N total at full throttle (hover at ~190 throttle)
    const double ARM_LENGTH = 0.22;    // m (distance from center to rotor)
    const double MAX_VEL = 1500.0;     // rad/s for visual rotor spin

public:
    ThejasSitlPlugin() : sock_fd(-1) {}

    ~ThejasSitlPlugin() {
        running = false;
        if (sock_fd >= 0) close(sock_fd);
        if (rx_thread.joinable()) rx_thread.join();
    }

    void Load(physics::ModelPtr _model, sdf::ElementPtr _sdf) override {
        model = _model;
        base_link = model->GetLink("base_link");

        motors[0] = model->GetJoint("rotor_0_joint");  // Rear Right (CW)
        motors[1] = model->GetJoint("rotor_1_joint");  // Front Right (CCW)
        motors[2] = model->GetJoint("rotor_2_joint");  // Rear Left (CCW)
        motors[3] = model->GetJoint("rotor_3_joint");  // Front Left (CW)

        sensors::SensorPtr s = sensors::SensorManager::Instance()->GetSensor("imu_sensor");
        imu_sensor = std::dynamic_pointer_cast<sensors::ImuSensor>(s);
        
        if (imu_sensor) {
            std::cerr << "[PLUGIN] IMU sensor found and active!" << std::endl;
        } else {
            std::cerr << "[PLUGIN] IMU sensor NOT FOUND!" << std::endl;
            // List all sensors
            std::vector<sensors::SensorPtr> all_sensors = sensors::SensorManager::Instance()->GetSensors();
            for (auto& sen : all_sensors) {
                std::cerr << "[PLUGIN] Available sensor: " << sen->Name() << " type: " << sen->Type() << std::endl;
            }
        }

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

        ignition::math::Vector3d lin_acc(0, 0, GRAVITY);
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

        ssize_t sent = sendto(sock_fd, imu_pkt, sizeof(imu_pkt), 0,
               (struct sockaddr*)&bridge_addr, sizeof(bridge_addr));
        static int udp_debug = 0;
        if (++udp_debug % 250 == 0) {
            std::cerr << "[PLUGIN] UDP sent: " << sent << " bytes to port 9002, IMU: gx=" << ang_vel.X() << " gy=" << ang_vel.Y() << " gz=" << ang_vel.Z() << std::endl;
        }

        std::lock_guard<std::mutex> lock(cmd_mutex);

        // Spin rotors for visual effect
        for (int i = 0; i < 4; i++) {
            if (motors[i]) {
                motors[i]->SetVelocity(0, motor_cmd[i] * MAX_VEL);
            }
        }

        // Apply forces/torques to base_link for actual flight physics
        if (base_link) {
            // Motor layout (X configuration):
            //   M1 (RR, CW)    M2 (FR, CCW)
            //        \          /
            //         \        /
            //          \      /
            //           \    /
            //            \  /
            //             \/
            //             /\
            //            /  \
            //           /    \
            //          /      \
            //         /        \
            //        /          \
            //   M3 (RL, CCW)   M4 (FL, CW)

            // Thrust from each motor (0 to MAX_THRUST_PER_MOTOR)
            double f1 = motor_cmd[0] * MAX_THRUST_PER_MOTOR;  // RR
            double f2 = motor_cmd[1] * MAX_THRUST_PER_MOTOR;  // FR
            double f3 = motor_cmd[2] * MAX_THRUST_PER_MOTOR;  // RL
            double f4 = motor_cmd[3] * MAX_THRUST_PER_MOTOR;  // FL

            // Total upward thrust
            double total_thrust = f1 + f2 + f3 + f4;

            // Torques (X configuration)
            // Roll: (f2 + f4) - (f1 + f3)  -- right motors vs left motors
            // Pitch: (f1 + f2) - (f3 + f4) -- front motors vs rear motors
            // Yaw: (f2 + f3) - (f1 + f4)   -- CCW motors (M2,M3) vs CW motors (M1,M4) drag torque
            double roll_torque  = ARM_LENGTH * ((f2 + f4) - (f1 + f3));
            double pitch_torque = ARM_LENGTH * ((f1 + f2) - (f3 + f4));
            double yaw_torque   = 0.015 * ((f2 + f3) - (f1 + f4));  // Yaw from motor drag (corrected)

            // Apply force in body frame (upward in body Z)
            ignition::math::Vector3d force_body(0, 0, total_thrust);

            // Convert to world frame using model's pose
            ignition::math::Pose3d pose = base_link->WorldPose();
            ignition::math::Vector3d force_world = pose.Rot().RotateVector(force_body);

            // Apply torques in body frame
            ignition::math::Vector3d torque_body(roll_torque, pitch_torque, yaw_torque);
            ignition::math::Vector3d torque_world = pose.Rot().RotateVector(torque_body);

            // Angular velocity damping for stability - stronger on yaw
            ignition::math::Vector3d ang_vel_world = base_link->WorldAngularVel();
            ignition::math::Vector3d damping_torque = ignition::math::Vector3d(
                -5.0 * ang_vel_world.X(),
                -5.0 * ang_vel_world.Y(),
                -10.0 * ang_vel_world.Z()  // Strong yaw damping
            );

            // Linear velocity damping (drag) - stronger to prevent drift
            ignition::math::Vector3d lin_vel_world = base_link->WorldLinearVel();
            ignition::math::Vector3d drag_force = -2.0 * lin_vel_world;

            // Ground effect: extra lift near ground
            double height = pose.Pos().Z();
            double ground_effect = 1.0;
            if (height < 1.0) {
                ground_effect = 1.0 + (1.0 - height) * 0.3;  // Up to 30% boost at ground
            }
            force_world *= ground_effect;

            // Position spring to origin (prevents drift) - weak spring
            ignition::math::Vector3d pos_world = pose.Pos();
            ignition::math::Vector3d spring_force = -0.5 * ignition::math::Vector3d(pos_world.X(), pos_world.Y(), 0);

            base_link->AddForce(force_world + drag_force + spring_force);
            base_link->AddTorque(torque_world + damping_torque);

            // Debug: print every 100 updates
            static int debug_counter = 0;
            if (++debug_counter % 100 == 0) {
                std::cerr << "[PLUGIN] Motors: " << motor_cmd[0] << " " << motor_cmd[1] << " " 
                          << motor_cmd[2] << " " << motor_cmd[3] 
                          << " | Thrust: " << total_thrust << "N" << std::endl;
            }
        }
    }
};

GZ_REGISTER_MODEL_PLUGIN(ThejasSitlPlugin)

}