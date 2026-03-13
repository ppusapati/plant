import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:flutter_joystick/flutter_joystick.dart';

import '../../../core/constants/protocol.dart';
import '../../../core/theme/app_theme.dart';
import '../../blocs/control/control_bloc.dart';
import '../../blocs/telemetry/telemetry_bloc.dart';

class ControlScreen extends StatelessWidget {
  const ControlScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return BlocBuilder<ControlBloc, ControlState>(
      builder: (context, controlState) {
        return BlocBuilder<TelemetryBloc, TelemetryState>(
          builder: (context, telemetryState) {
            final t = telemetryState.telemetry;

            return Padding(
              padding: const EdgeInsets.all(16),
              child: Column(
                children: [
                  // Mode selector
                  _ModeSelector(currentMode: controlState.mode),
                  const SizedBox(height: 16),

                  // Main control area
                  Expanded(
                    child: Row(
                      children: [
                        // Left: Joystick
                        Expanded(
                          flex: 2,
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              Text(
                                'Throttle: ${(controlState.throttle * 100).toStringAsFixed(0)}%  '
                                'Steering: ${(controlState.steering * 100).toStringAsFixed(0)}%',
                                style: const TextStyle(color: AppColors.textSecondary, fontSize: 12),
                              ),
                              const SizedBox(height: 12),
                              SizedBox(
                                width: 200,
                                height: 200,
                                child: Joystick(
                                  mode: JoystickMode.all,
                                  listener: (details) {
                                    context.read<ControlBloc>().add(
                                      JoystickUpdate(-details.y, details.x),
                                    );
                                  },
                                ),
                              ),
                            ],
                          ),
                        ),

                        // Center: Speed & telemetry
                        Expanded(
                          flex: 1,
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              // Speed display
                              Text(
                                '${t.position.speed.toStringAsFixed(1)}',
                                style: const TextStyle(fontSize: 48, fontWeight: FontWeight.bold),
                              ),
                              const Text('m/s', style: TextStyle(color: AppColors.textSecondary)),
                              const SizedBox(height: 24),

                              // Max speed slider
                              Column(
                                children: [
                                  const Text('Max Speed', style: TextStyle(fontSize: 12, color: AppColors.textSecondary)),
                                  Slider(
                                    value: controlState.maxSpeed,
                                    min: 0.1,
                                    max: 2.0,
                                    divisions: 19,
                                    label: '${controlState.maxSpeed.toStringAsFixed(1)} m/s',
                                    onChanged: (v) => context.read<ControlBloc>().add(SetMaxSpeed(v)),
                                  ),
                                ],
                              ),
                            ],
                          ),
                        ),

                        // Right: Action buttons
                        Expanded(
                          flex: 1,
                          child: Column(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              // E-STOP button
                              _EStopButton(active: controlState.estopActive),
                              const SizedBox(height: 24),

                              // Deseeding toggle
                              _ActionButton(
                                icon: Icons.grass,
                                label: controlState.deseedingActive ? 'Stop Deseed' : 'Start Deseed',
                                color: controlState.deseedingActive ? AppColors.error : AppColors.primary,
                                onPressed: () {
                                  if (controlState.deseedingActive) {
                                    context.read<ControlBloc>().add(const StopDeseeding());
                                  } else {
                                    context.read<ControlBloc>().add(const StartDeseeding());
                                  }
                                },
                              ),
                              const SizedBox(height: 12),

                              // Probe deploy/retract
                              _ActionButton(
                                icon: controlState.probesDeployed ? Icons.publish : Icons.download,
                                label: controlState.probesDeployed ? 'Retract' : 'Deploy Probes',
                                color: AppColors.info,
                                onPressed: () {
                                  if (controlState.probesDeployed) {
                                    context.read<ControlBloc>().add(const RetractProbes());
                                  } else {
                                    context.read<ControlBloc>().add(const DeployProbes());
                                  }
                                },
                              ),
                            ],
                          ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            );
          },
        );
      },
    );
  }
}

class _ModeSelector extends StatelessWidget {
  final RobotMode currentMode;
  const _ModeSelector({required this.currentMode});

  @override
  Widget build(BuildContext context) {
    final modes = [RobotMode.idle, RobotMode.manual, RobotMode.autonomous, RobotMode.returnHome];

    return Row(
      children: modes.map((mode) {
        final selected = mode == currentMode;
        return Expanded(
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: ChoiceChip(
              label: Text(mode.label),
              selected: selected,
              onSelected: (_) => context.read<ControlBloc>().add(ChangeMode(mode)),
              selectedColor: AppColors.primary,
            ),
          ),
        );
      }).toList(),
    );
  }
}

class _EStopButton extends StatelessWidget {
  final bool active;
  const _EStopButton({required this.active});

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: () {
        HapticFeedback.heavyImpact();
        context.read<ControlBloc>().add(const EmergencyStopPressed());
      },
      child: Container(
        width: 80,
        height: 80,
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          color: active ? AppColors.error : AppColors.error.withValues(alpha: 0.8),
          border: Border.all(color: Colors.white, width: 3),
          boxShadow: [
            BoxShadow(
              color: AppColors.error.withValues(alpha: active ? 0.6 : 0.3),
              blurRadius: active ? 20 : 10,
              spreadRadius: active ? 4 : 2,
            ),
          ],
        ),
        child: const Center(
          child: Text(
            'E-STOP',
            style: TextStyle(
              color: Colors.white,
              fontWeight: FontWeight.bold,
              fontSize: 12,
            ),
          ),
        ),
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final Color color;
  final VoidCallback onPressed;

  const _ActionButton({
    required this.icon,
    required this.label,
    required this.color,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: double.infinity,
      child: ElevatedButton.icon(
        onPressed: onPressed,
        icon: Icon(icon, size: 18),
        label: Text(label, style: const TextStyle(fontSize: 12)),
        style: ElevatedButton.styleFrom(
          backgroundColor: color,
          foregroundColor: Colors.white,
          padding: const EdgeInsets.symmetric(vertical: 12),
        ),
      ),
    );
  }
}
