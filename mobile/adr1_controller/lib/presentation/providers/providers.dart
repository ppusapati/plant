import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../data/repositories/robot_repository_impl.dart';
import '../../domain/repositories/robot_repository.dart';
import '../blocs/connection/connection_bloc.dart';
import '../blocs/control/control_bloc.dart';
import '../blocs/telemetry/telemetry_bloc.dart';

/// Singleton robot repository
final robotRepositoryProvider = Provider<RobotRepository>((ref) {
  final repo = RobotRepositoryImpl();
  ref.onDispose(() => repo.dispose());
  return repo;
});

/// Connection BLoC - manages WiFi/BLE connection state machine
final connectionBlocProvider = Provider<ConnectionBloc>((ref) {
  final repo = ref.watch(robotRepositoryProvider);
  final bloc = ConnectionBloc(repo);
  ref.onDispose(() => bloc.close());
  return bloc;
});

/// Control BLoC - manages manual control, e-stop, mode switching
final controlBlocProvider = Provider<ControlBloc>((ref) {
  final repo = ref.watch(robotRepositoryProvider);
  final bloc = ControlBloc(repo);
  ref.onDispose(() => bloc.close());
  return bloc;
});

/// Telemetry BLoC - processes incoming telemetry and sensor data
final telemetryBlocProvider = Provider<TelemetryBloc>((ref) {
  final repo = ref.watch(robotRepositoryProvider);
  final bloc = TelemetryBloc(repo)..add(const StartListening());
  ref.onDispose(() => bloc.close());
  return bloc;
});

/// Connection state stream
final connectionStateProvider = StreamProvider<ConnectionState>((ref) {
  final bloc = ref.watch(connectionBlocProvider);
  return bloc.stream;
});

/// Telemetry state stream
final telemetryStateProvider = StreamProvider<TelemetryState>((ref) {
  final bloc = ref.watch(telemetryBlocProvider);
  return bloc.stream;
});

/// Control state stream
final controlStateProvider = StreamProvider<ControlState>((ref) {
  final bloc = ref.watch(controlBlocProvider);
  return bloc.stream;
});
