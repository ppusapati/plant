import 'dart:async';
import 'package:bloc/bloc.dart';
import 'package:equatable/equatable.dart';

import '../../../data/models/telemetry.dart';
import '../../../data/models/sensor_data.dart';
import '../../../data/repositories/robot_repository_impl.dart';
import '../../../domain/repositories/robot_repository.dart';

// --- Events ---

sealed class TelemetryEvent extends Equatable {
  const TelemetryEvent();
  @override
  List<Object?> get props => [];
}

class StartListening extends TelemetryEvent {
  const StartListening();
}

class StopListening extends TelemetryEvent {
  const StopListening();
}

class TelemetryReceived extends TelemetryEvent {
  final RobotTelemetry telemetry;
  const TelemetryReceived(this.telemetry);
  @override
  List<Object?> get props => [telemetry];
}

class SensorDataReceived extends TelemetryEvent {
  final SensorUpdate update;
  const SensorDataReceived(this.update);
  @override
  List<Object?> get props => [];
}

// --- State ---

class TelemetryState extends Equatable {
  final RobotTelemetry telemetry;
  final NdviData? latestNdvi;
  final SoilData? latestSoil;
  final EnvironmentData? latestEnvironment;
  final List<PositionData> positionHistory;
  final List<BatteryData> batteryHistory;

  const TelemetryState({
    required this.telemetry,
    this.latestNdvi,
    this.latestSoil,
    this.latestEnvironment,
    this.positionHistory = const [],
    this.batteryHistory = const [],
  });

  factory TelemetryState.initial() => TelemetryState(
    telemetry: RobotTelemetry.empty(),
  );

  TelemetryState copyWith({
    RobotTelemetry? telemetry,
    NdviData? latestNdvi,
    SoilData? latestSoil,
    EnvironmentData? latestEnvironment,
    List<PositionData>? positionHistory,
    List<BatteryData>? batteryHistory,
  }) => TelemetryState(
    telemetry: telemetry ?? this.telemetry,
    latestNdvi: latestNdvi ?? this.latestNdvi,
    latestSoil: latestSoil ?? this.latestSoil,
    latestEnvironment: latestEnvironment ?? this.latestEnvironment,
    positionHistory: positionHistory ?? this.positionHistory,
    batteryHistory: batteryHistory ?? this.batteryHistory,
  );

  @override
  List<Object?> get props => [telemetry, latestNdvi, latestSoil, latestEnvironment];
}

// --- Bloc ---

class TelemetryBloc extends Bloc<TelemetryEvent, TelemetryState> {
  final RobotRepository _repo;
  StreamSubscription? _telemetrySub;
  StreamSubscription? _sensorSub;

  static const _maxHistorySize = 300; // ~30 seconds at 10Hz

  TelemetryBloc(this._repo) : super(TelemetryState.initial()) {
    on<StartListening>(_onStartListening);
    on<StopListening>(_onStopListening);
    on<TelemetryReceived>(_onTelemetryReceived);
    on<SensorDataReceived>(_onSensorDataReceived);
  }

  void _onStartListening(StartListening event, Emitter<TelemetryState> emit) {
    _telemetrySub = _repo.telemetryStream.listen(
      (t) => add(TelemetryReceived(t)),
    );
    _sensorSub = _repo.sensorStream.listen(
      (s) => add(SensorDataReceived(s)),
    );
  }

  void _onStopListening(StopListening event, Emitter<TelemetryState> emit) {
    _telemetrySub?.cancel();
    _sensorSub?.cancel();
  }

  void _onTelemetryReceived(TelemetryReceived event, Emitter<TelemetryState> emit) {
    final posHistory = List<PositionData>.from(state.positionHistory)
      ..add(event.telemetry.position);
    if (posHistory.length > _maxHistorySize) posHistory.removeAt(0);

    final battHistory = List<BatteryData>.from(state.batteryHistory)
      ..add(event.telemetry.battery);
    if (battHistory.length > _maxHistorySize) battHistory.removeAt(0);

    emit(state.copyWith(
      telemetry: event.telemetry,
      positionHistory: posHistory,
      batteryHistory: battHistory,
    ));
  }

  void _onSensorDataReceived(SensorDataReceived event, Emitter<TelemetryState> emit) {
    switch (event.update) {
      case NdviUpdate(:final data):
        emit(state.copyWith(latestNdvi: data));
      case SoilUpdate(:final data):
        emit(state.copyWith(latestSoil: data));
      case EnvironmentUpdate(:final data):
        emit(state.copyWith(latestEnvironment: data));
    }
  }

  @override
  Future<void> close() {
    _telemetrySub?.cancel();
    _sensorSub?.cancel();
    return super.close();
  }
}
