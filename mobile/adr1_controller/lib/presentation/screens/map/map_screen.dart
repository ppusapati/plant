import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_map/flutter_map.dart';
import 'package:latlong2/latlong.dart';

import '../../../core/theme/app_theme.dart';
import '../../../data/models/mission.dart';
import '../../blocs/telemetry/telemetry_bloc.dart';
import '../../blocs/control/control_bloc.dart';
import '../../../core/constants/protocol.dart';

class MapScreen extends StatefulWidget {
  const MapScreen({super.key});

  @override
  State<MapScreen> createState() => _MapScreenState();
}

class _MapScreenState extends State<MapScreen> {
  final MapController _mapController = MapController();
  final List<Waypoint> _waypoints = [];
  final List<LatLng> _boundary = [];
  bool _editingBoundary = false;
  bool _editingWaypoints = false;

  @override
  Widget build(BuildContext context) {
    return BlocBuilder<TelemetryBloc, TelemetryState>(
      builder: (context, state) {
        final pos = state.telemetry.position;
        final robotPos = LatLng(pos.latitude, pos.longitude);

        // Build trail from position history
        final trail = state.positionHistory
            .where((p) => p.fixValid)
            .map((p) => LatLng(p.latitude, p.longitude))
            .toList();

        return Stack(
          children: [
            FlutterMap(
              mapController: _mapController,
              options: MapOptions(
                initialCenter: pos.fixValid ? robotPos : const LatLng(20.5937, 78.9629), // Default: India
                initialZoom: 18,
                maxZoom: 22,
                onTap: (tapPos, latLng) => _onMapTap(latLng),
              ),
              children: [
                TileLayer(
                  urlTemplate: 'https://tile.openstreetmap.org/{z}/{x}/{y}.png',
                  userAgentPackageName: 'com.adr1.controller',
                ),

                // Field boundary polygon
                if (_boundary.length >= 3)
                  PolygonLayer(
                    polygons: [
                      Polygon(
                        points: _boundary,
                        color: AppColors.primary.withValues(alpha: 0.15),
                        borderColor: AppColors.primary,
                        borderStrokeWidth: 2,
                        isFilled: true,
                      ),
                    ],
                  ),

                // Robot trail
                if (trail.length >= 2)
                  PolylineLayer(
                    polylines: [
                      Polyline(
                        points: trail,
                        color: AppColors.accent.withValues(alpha: 0.6),
                        strokeWidth: 3,
                      ),
                    ],
                  ),

                // Waypoint lines
                if (_waypoints.length >= 2)
                  PolylineLayer(
                    polylines: [
                      Polyline(
                        points: _waypoints.map((w) => w.position).toList(),
                        color: AppColors.info,
                        strokeWidth: 2,
                        isDotted: true,
                      ),
                    ],
                  ),

                // Waypoint markers
                MarkerLayer(
                  markers: [
                    // Robot position
                    if (pos.fixValid)
                      Marker(
                        point: robotPos,
                        width: 40,
                        height: 40,
                        child: Transform.rotate(
                          angle: pos.heading * 3.14159 / 180,
                          child: const Icon(Icons.navigation, color: AppColors.accent, size: 36),
                        ),
                      ),

                    // Waypoints
                    ..._waypoints.asMap().entries.map((e) => Marker(
                      point: e.value.position,
                      width: 30,
                      height: 30,
                      child: Container(
                        decoration: BoxDecoration(
                          color: _waypointColor(e.value.action),
                          shape: BoxShape.circle,
                          border: Border.all(color: Colors.white, width: 2),
                        ),
                        child: Center(
                          child: Text(
                            '${e.key + 1}',
                            style: const TextStyle(color: Colors.white, fontSize: 10, fontWeight: FontWeight.bold),
                          ),
                        ),
                      ),
                    )),

                    // Boundary vertices
                    ..._boundary.map((p) => Marker(
                      point: p,
                      width: 16,
                      height: 16,
                      child: Container(
                        decoration: BoxDecoration(
                          color: AppColors.primary,
                          shape: BoxShape.circle,
                          border: Border.all(color: Colors.white, width: 1.5),
                        ),
                      ),
                    )),
                  ],
                ),
              ],
            ),

            // Map controls
            Positioned(
              right: 16,
              top: 16,
              child: Column(
                children: [
                  _MapButton(
                    icon: Icons.my_location,
                    onPressed: () {
                      if (pos.fixValid) {
                        _mapController.move(robotPos, _mapController.camera.zoom);
                      }
                    },
                  ),
                  const SizedBox(height: 8),
                  _MapButton(
                    icon: Icons.crop_square,
                    color: _editingBoundary ? AppColors.primary : null,
                    onPressed: () => setState(() {
                      _editingBoundary = !_editingBoundary;
                      _editingWaypoints = false;
                    }),
                    tooltip: 'Draw field boundary',
                  ),
                  const SizedBox(height: 8),
                  _MapButton(
                    icon: Icons.pin_drop,
                    color: _editingWaypoints ? AppColors.info : null,
                    onPressed: () => setState(() {
                      _editingWaypoints = !_editingWaypoints;
                      _editingBoundary = false;
                    }),
                    tooltip: 'Add waypoints',
                  ),
                  const SizedBox(height: 8),
                  _MapButton(
                    icon: Icons.clear_all,
                    onPressed: _clearAll,
                    tooltip: 'Clear all',
                  ),
                ],
              ),
            ),

            // Mission panel
            if (_waypoints.isNotEmpty)
              Positioned(
                left: 16,
                bottom: 16,
                child: Card(
                  child: Padding(
                    padding: const EdgeInsets.all(12),
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text('${_waypoints.length} waypoints', style: const TextStyle(fontWeight: FontWeight.bold)),
                        const SizedBox(height: 8),
                        Row(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            ElevatedButton.icon(
                              onPressed: _startMission,
                              icon: const Icon(Icons.play_arrow, size: 18),
                              label: const Text('Start'),
                              style: ElevatedButton.styleFrom(backgroundColor: AppColors.primary),
                            ),
                            const SizedBox(width: 8),
                            OutlinedButton(
                              onPressed: () => setState(() => _waypoints.clear()),
                              child: const Text('Clear'),
                            ),
                          ],
                        ),
                      ],
                    ),
                  ),
                ),
              ),
          ],
        );
      },
    );
  }

  void _onMapTap(LatLng point) {
    setState(() {
      if (_editingBoundary) {
        _boundary.add(point);
      } else if (_editingWaypoints) {
        _waypoints.add(Waypoint(
          id: 'wp_${_waypoints.length}',
          position: point,
          action: WaypointAction.navigate,
        ));
      }
    });
  }

  void _clearAll() {
    setState(() {
      _waypoints.clear();
      _boundary.clear();
    });
  }

  void _startMission() {
    if (_waypoints.isEmpty) return;
    context.read<ControlBloc>().add(const ChangeMode(RobotMode.autonomous));
    // Mission upload handled via command protocol
  }

  Color _waypointColor(WaypointAction action) => switch (action) {
    WaypointAction.navigate => AppColors.info,
    WaypointAction.sampleSoil => Colors.brown,
    WaypointAction.scanNdvi => AppColors.ndviHealthy,
    WaypointAction.deseed => AppColors.primary,
    WaypointAction.photograph => Colors.purple,
  };
}

class _MapButton extends StatelessWidget {
  final IconData icon;
  final VoidCallback onPressed;
  final Color? color;
  final String? tooltip;

  const _MapButton({required this.icon, required this.onPressed, this.color, this.tooltip});

  @override
  Widget build(BuildContext context) {
    return FloatingActionButton.small(
      heroTag: null,
      onPressed: onPressed,
      tooltip: tooltip,
      backgroundColor: color ?? AppColors.surface,
      child: Icon(icon, size: 20),
    );
  }
}
