import 'package:equatable/equatable.dart';
import 'package:latlong2/latlong.dart';

/// A waypoint in a mission plan
class Waypoint extends Equatable {
  final String id;
  final LatLng position;
  final double? altitude;
  final WaypointAction action;
  final double speedMps;       // target speed in m/s
  final int dwellSeconds;      // time to stay at waypoint

  const Waypoint({
    required this.id,
    required this.position,
    this.altitude,
    this.action = WaypointAction.navigate,
    this.speedMps = 0.5,
    this.dwellSeconds = 0,
  });

  Map<String, dynamic> toJson() => {
    'id': id,
    'lat': position.latitude,
    'lng': position.longitude,
    'alt': altitude,
    'action': action.index,
    'speed': speedMps,
    'dwell': dwellSeconds,
  };

  factory Waypoint.fromJson(Map<String, dynamic> json) => Waypoint(
    id: json['id'] as String,
    position: LatLng(json['lat'] as double, json['lng'] as double),
    altitude: json['alt'] as double?,
    action: WaypointAction.values[json['action'] as int],
    speedMps: json['speed'] as double,
    dwellSeconds: json['dwell'] as int,
  );

  @override
  List<Object?> get props => [id, position, altitude, action, speedMps, dwellSeconds];
}

enum WaypointAction {
  navigate,        // Just move to this point
  sampleSoil,      // Deploy probes and take soil reading
  scanNdvi,        // Extended NDVI scan at this location
  deseed,          // Activate deseeding mechanism
  photograph,      // Take high-res photo for analysis
}

/// A complete mission plan
class MissionPlan extends Equatable {
  final String id;
  final String name;
  final DateTime created;
  final List<Waypoint> waypoints;
  final MissionPattern pattern;
  final double rowSpacingM;
  final List<LatLng> boundary;  // Field boundary polygon

  const MissionPlan({
    required this.id,
    required this.name,
    required this.created,
    required this.waypoints,
    this.pattern = MissionPattern.serpentine,
    this.rowSpacingM = 1.0,
    this.boundary = const [],
  });

  double get totalDistanceM {
    double dist = 0;
    const distance = Distance();
    for (int i = 1; i < waypoints.length; i++) {
      dist += distance.as(
        LengthUnit.Meter,
        waypoints[i - 1].position,
        waypoints[i].position,
      );
    }
    return dist;
  }

  int get estimatedMinutes => (totalDistanceM / 0.5 / 60).ceil(); // at 0.5 m/s

  Map<String, dynamic> toJson() => {
    'id': id,
    'name': name,
    'created': created.toIso8601String(),
    'waypoints': waypoints.map((w) => w.toJson()).toList(),
    'pattern': pattern.index,
    'rowSpacing': rowSpacingM,
    'boundary': boundary.map((p) => {'lat': p.latitude, 'lng': p.longitude}).toList(),
  };

  factory MissionPlan.fromJson(Map<String, dynamic> json) => MissionPlan(
    id: json['id'] as String,
    name: json['name'] as String,
    created: DateTime.parse(json['created'] as String),
    waypoints: (json['waypoints'] as List).map((w) => Waypoint.fromJson(w as Map<String, dynamic>)).toList(),
    pattern: MissionPattern.values[json['pattern'] as int],
    rowSpacingM: json['rowSpacing'] as double,
    boundary: (json['boundary'] as List).map((p) => LatLng(p['lat'] as double, p['lng'] as double)).toList(),
  );

  @override
  List<Object?> get props => [id, name, created, waypoints, pattern, rowSpacingM, boundary];
}

enum MissionPattern {
  serpentine,     // Back and forth rows (boustrophedon)
  spiral,         // Spiral inward
  perimeter,      // Follow boundary
  custom,         // Manual waypoints
}
