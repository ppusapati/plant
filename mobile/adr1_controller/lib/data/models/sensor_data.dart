import 'package:equatable/equatable.dart';

/// NDVI and plant health data from dual AR0234CS cameras
class NdviData extends Equatable {
  final DateTime timestamp;
  final double meanNdvi;           // 0.0 - 1.0
  final double minNdvi;
  final double maxNdvi;
  final double healthyPercent;     // % of area NDVI > 0.4
  final double stressedPercent;    // % of area 0.2 < NDVI < 0.4
  final double barePercent;        // % of area NDVI < 0.2
  final List<NdviGridCell> grid;   // 8x8 grid cells

  const NdviData({
    required this.timestamp,
    required this.meanNdvi,
    required this.minNdvi,
    required this.maxNdvi,
    required this.healthyPercent,
    required this.stressedPercent,
    required this.barePercent,
    this.grid = const [],
  });

  factory NdviData.empty() => NdviData(
    timestamp: DateTime.now(),
    meanNdvi: 0, minNdvi: 0, maxNdvi: 0,
    healthyPercent: 0, stressedPercent: 0, barePercent: 0,
  );

  @override
  List<Object?> get props => [timestamp, meanNdvi, minNdvi, maxNdvi, healthyPercent, stressedPercent, barePercent];
}

class NdviGridCell {
  final int row;
  final int col;
  final double ndvi;

  const NdviGridCell({required this.row, required this.col, required this.ndvi});
}

/// Soil probe measurements
class SoilData extends Equatable {
  final DateTime timestamp;
  final double latitude;
  final double longitude;
  final double nitrogen;       // mg/kg
  final double phosphorus;     // mg/kg
  final double potassium;      // mg/kg
  final double ph;             // 0-14
  final double ec;             // µS/cm
  final double moisture;       // % VWC
  final double temperature;    // °C

  const SoilData({
    required this.timestamp,
    required this.latitude,
    required this.longitude,
    required this.nitrogen,
    required this.phosphorus,
    required this.potassium,
    required this.ph,
    required this.ec,
    required this.moisture,
    required this.temperature,
  });

  factory SoilData.empty() => SoilData(
    timestamp: DateTime.now(),
    latitude: 0, longitude: 0,
    nitrogen: 0, phosphorus: 0, potassium: 0,
    ph: 7, ec: 0, moisture: 0, temperature: 25,
  );

  /// Overall soil health score (0-100)
  double get healthScore {
    double score = 0;
    // pH: optimal 6.0-7.0
    score += _rangeScore(ph, 6.0, 7.0, 3.0, 9.0) * 20;
    // Moisture: optimal 30-60%
    score += _rangeScore(moisture, 30, 60, 0, 100) * 20;
    // NPK scoring (higher is generally better up to a point)
    score += _rangeScore(nitrogen, 40, 200, 0, 500) * 20;
    score += _rangeScore(phosphorus, 20, 100, 0, 300) * 20;
    score += _rangeScore(potassium, 100, 300, 0, 600) * 20;
    return score.clamp(0, 100);
  }

  static double _rangeScore(double val, double optMin, double optMax, double absMin, double absMax) {
    if (val >= optMin && val <= optMax) return 1.0;
    if (val < absMin || val > absMax) return 0.0;
    if (val < optMin) return (val - absMin) / (optMin - absMin);
    return (absMax - val) / (absMax - optMax);
  }

  @override
  List<Object?> get props => [timestamp, nitrogen, phosphorus, potassium, ph, ec, moisture, temperature];
}

/// Environmental sensor readings from BME280, BH1750, VEML6075
class EnvironmentData extends Equatable {
  final DateTime timestamp;
  final double temperature;     // °C (BME280)
  final double humidity;        // % RH (BME280)
  final double pressure;        // hPa (BME280)
  final double lightLux;        // lux (BH1750)
  final double uvIndex;         // 0-15 (VEML6075)
  final double windSpeed;       // m/s (anemometer)
  final double rainfall;        // mm/h (rain gauge)
  final double leafTemp;        // °C (MLX90614)

  const EnvironmentData({
    required this.timestamp,
    required this.temperature,
    required this.humidity,
    required this.pressure,
    required this.lightLux,
    required this.uvIndex,
    required this.windSpeed,
    required this.rainfall,
    required this.leafTemp,
  });

  factory EnvironmentData.empty() => EnvironmentData(
    timestamp: DateTime.now(),
    temperature: 0, humidity: 0, pressure: 1013,
    lightLux: 0, uvIndex: 0, windSpeed: 0, rainfall: 0, leafTemp: 0,
  );

  /// Leaf temperature stress indicator
  double get leafStressDelta => leafTemp - temperature;
  bool get waterStress => leafStressDelta > 5.0;

  @override
  List<Object?> get props => [timestamp, temperature, humidity, pressure, lightLux, uvIndex, windSpeed, rainfall, leafTemp];
}

/// Composite health score combining all sensors
class HealthReport extends Equatable {
  final DateTime timestamp;
  final double overallScore;     // 0-100
  final double ndviScore;        // 0-100
  final double soilScore;        // 0-100
  final double environmentScore; // 0-100
  final List<String> alerts;

  const HealthReport({
    required this.timestamp,
    required this.overallScore,
    required this.ndviScore,
    required this.soilScore,
    required this.environmentScore,
    this.alerts = const [],
  });

  factory HealthReport.empty() => HealthReport(
    timestamp: DateTime.now(),
    overallScore: 0, ndviScore: 0, soilScore: 0, environmentScore: 0,
  );

  @override
  List<Object?> get props => [timestamp, overallScore, ndviScore, soilScore, environmentScore, alerts];
}
