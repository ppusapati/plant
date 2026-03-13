import 'package:equatable/equatable.dart';

/// Complete robot telemetry state
class RobotTelemetry extends Equatable {
  final DateTime timestamp;
  final PositionData position;
  final AttitudeData attitude;
  final BatteryData battery;
  final MotorData motors;
  final ObstacleData obstacles;
  final MissionData? mission;
  final int rssi;
  final int mode;
  final bool connected;

  const RobotTelemetry({
    required this.timestamp,
    required this.position,
    required this.attitude,
    required this.battery,
    required this.motors,
    required this.obstacles,
    this.mission,
    this.rssi = 0,
    this.mode = 0,
    this.connected = false,
  });

  factory RobotTelemetry.empty() => RobotTelemetry(
    timestamp: DateTime.now(),
    position: PositionData.empty(),
    attitude: AttitudeData.empty(),
    battery: BatteryData.empty(),
    motors: MotorData.empty(),
    obstacles: ObstacleData.empty(),
  );

  RobotTelemetry copyWith({
    DateTime? timestamp,
    PositionData? position,
    AttitudeData? attitude,
    BatteryData? battery,
    MotorData? motors,
    ObstacleData? obstacles,
    MissionData? mission,
    int? rssi,
    int? mode,
    bool? connected,
  }) {
    return RobotTelemetry(
      timestamp: timestamp ?? this.timestamp,
      position: position ?? this.position,
      attitude: attitude ?? this.attitude,
      battery: battery ?? this.battery,
      motors: motors ?? this.motors,
      obstacles: obstacles ?? this.obstacles,
      mission: mission ?? this.mission,
      rssi: rssi ?? this.rssi,
      mode: mode ?? this.mode,
      connected: connected ?? this.connected,
    );
  }

  @override
  List<Object?> get props => [timestamp, position, attitude, battery, motors, obstacles, mission, rssi, mode, connected];
}

class PositionData extends Equatable {
  final double latitude;
  final double longitude;
  final double altitude;
  final double speed;         // m/s
  final double heading;       // degrees
  final int satellites;
  final double hdop;
  final bool fixValid;

  const PositionData({
    required this.latitude,
    required this.longitude,
    required this.altitude,
    required this.speed,
    required this.heading,
    required this.satellites,
    required this.hdop,
    required this.fixValid,
  });

  factory PositionData.empty() => const PositionData(
    latitude: 0, longitude: 0, altitude: 0,
    speed: 0, heading: 0, satellites: 0, hdop: 99, fixValid: false,
  );

  @override
  List<Object?> get props => [latitude, longitude, altitude, speed, heading, satellites, hdop, fixValid];
}

class AttitudeData extends Equatable {
  final double roll;
  final double pitch;
  final double yaw;
  final double angularVelocity;

  const AttitudeData({
    required this.roll,
    required this.pitch,
    required this.yaw,
    required this.angularVelocity,
  });

  factory AttitudeData.empty() => const AttitudeData(roll: 0, pitch: 0, yaw: 0, angularVelocity: 0);

  @override
  List<Object?> get props => [roll, pitch, yaw, angularVelocity];
}

class BatteryData extends Equatable {
  final double voltage;        // V
  final double current;        // A
  final double soc;            // 0-100%
  final double temperature;    // °C
  final double powerWatts;
  final int estimatedMinutes;
  final bool charging;

  const BatteryData({
    required this.voltage,
    required this.current,
    required this.soc,
    required this.temperature,
    required this.powerWatts,
    required this.estimatedMinutes,
    required this.charging,
  });

  factory BatteryData.empty() => const BatteryData(
    voltage: 0, current: 0, soc: 0, temperature: 0,
    powerWatts: 0, estimatedMinutes: 0, charging: false,
  );

  @override
  List<Object?> get props => [voltage, current, soc, temperature, powerWatts, estimatedMinutes, charging];
}

class MotorData extends Equatable {
  final List<double> currents;   // 4 motors, Amps
  final List<double> rpms;       // 4 motors
  final List<double> temps;      // 4 motors, °C
  final bool deseedingActive;
  final double deseedingRpm;

  const MotorData({
    required this.currents,
    required this.rpms,
    required this.temps,
    required this.deseedingActive,
    required this.deseedingRpm,
  });

  factory MotorData.empty() => const MotorData(
    currents: [0, 0, 0, 0], rpms: [0, 0, 0, 0], temps: [0, 0, 0, 0],
    deseedingActive: false, deseedingRpm: 0,
  );

  @override
  List<Object?> get props => [currents, rpms, temps, deseedingActive, deseedingRpm];
}

class ObstacleData extends Equatable {
  final double frontMm;
  final double rightMm;
  final double rearMm;
  final double leftMm;
  final bool emergencyStop;

  const ObstacleData({
    required this.frontMm,
    required this.rightMm,
    required this.rearMm,
    required this.leftMm,
    required this.emergencyStop,
  });

  factory ObstacleData.empty() => const ObstacleData(
    frontMm: 9999, rightMm: 9999, rearMm: 9999, leftMm: 9999, emergencyStop: false,
  );

  double get minDistance => [frontMm, rightMm, rearMm, leftMm]
      .reduce((a, b) => a < b ? a : b);

  @override
  List<Object?> get props => [frontMm, rightMm, rearMm, leftMm, emergencyStop];
}

class MissionData extends Equatable {
  final String missionId;
  final int totalWaypoints;
  final int currentWaypoint;
  final double progressPercent;
  final double areaCoveredSqM;
  final int elapsedSeconds;
  final bool paused;

  const MissionData({
    required this.missionId,
    required this.totalWaypoints,
    required this.currentWaypoint,
    required this.progressPercent,
    required this.areaCoveredSqM,
    required this.elapsedSeconds,
    required this.paused,
  });

  @override
  List<Object?> get props => [missionId, totalWaypoints, currentWaypoint, progressPercent, areaCoveredSqM, elapsedSeconds, paused];
}
