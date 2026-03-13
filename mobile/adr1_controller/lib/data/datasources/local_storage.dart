import 'dart:convert';
import 'package:hive_flutter/hive_flutter.dart';
import '../models/mission.dart';

/// Hive-based local storage for missions, settings, and cached data
class LocalStorage {
  static late Box _settingsBox;
  static late Box _missionsBox;
  static late Box _sensorLogBox;

  static Future<void> init() async {
    _settingsBox = await Hive.openBox('settings');
    _missionsBox = await Hive.openBox('missions');
    _sensorLogBox = await Hive.openBox('sensor_log');
  }

  // --- Settings ---

  static String? getLastRobotAddress() => _settingsBox.get('lastRobotAddress') as String?;
  static Future<void> setLastRobotAddress(String addr) => _settingsBox.put('lastRobotAddress', addr);

  static String getConnectionType() => _settingsBox.get('connectionType', defaultValue: 'wifi') as String;
  static Future<void> setConnectionType(String type) => _settingsBox.put('connectionType', type);

  static double getMaxSpeed() => _settingsBox.get('maxSpeed', defaultValue: 1.0) as double;
  static Future<void> setMaxSpeed(double speed) => _settingsBox.put('maxSpeed', speed);

  static bool getVibrationEnabled() => _settingsBox.get('vibrationEnabled', defaultValue: true) as bool;
  static Future<void> setVibrationEnabled(bool enabled) => _settingsBox.put('vibrationEnabled', enabled);

  static int getTelemetryRateHz() => _settingsBox.get('telemetryRate', defaultValue: 10) as int;
  static Future<void> setTelemetryRateHz(int hz) => _settingsBox.put('telemetryRate', hz);

  // --- Missions ---

  static List<MissionPlan> getSavedMissions() {
    final keys = _missionsBox.keys.toList();
    return keys.map((key) {
      final json = jsonDecode(_missionsBox.get(key) as String) as Map<String, dynamic>;
      return MissionPlan.fromJson(json);
    }).toList();
  }

  static Future<void> saveMission(MissionPlan mission) =>
      _missionsBox.put(mission.id, jsonEncode(mission.toJson()));

  static Future<void> deleteMission(String id) => _missionsBox.delete(id);

  // --- Sensor Log ---

  static Future<void> logSensorReading(String type, Map<String, dynamic> data) {
    final key = '${type}_${DateTime.now().millisecondsSinceEpoch}';
    return _sensorLogBox.put(key, jsonEncode(data));
  }

  static Future<void> clearOldLogs({int keepDays = 7}) async {
    final cutoff = DateTime.now().subtract(Duration(days: keepDays)).millisecondsSinceEpoch;
    final keysToDelete = _sensorLogBox.keys.where((key) {
      final ts = int.tryParse((key as String).split('_').last) ?? 0;
      return ts < cutoff;
    }).toList();
    for (final key in keysToDelete) {
      await _sensorLogBox.delete(key);
    }
  }
}
