import 'dart:async';
import 'package:bloc/bloc.dart';
import 'package:equatable/equatable.dart';

import '../../../core/constants/protocol.dart';
import '../../../domain/repositories/robot_repository.dart';

// --- Events ---

sealed class ControlEvent extends Equatable {
  const ControlEvent();
  @override
  List<Object?> get props => [];
}

class JoystickUpdate extends ControlEvent {
  final double throttle;  // -1.0 (reverse) to 1.0 (forward)
  final double steering;  // -1.0 (left) to 1.0 (right)
  const JoystickUpdate(this.throttle, this.steering);
  @override
  List<Object?> get props => [throttle, steering];
}

class EmergencyStopPressed extends ControlEvent {
  const EmergencyStopPressed();
}

class ChangeMode extends ControlEvent {
  final RobotMode mode;
  const ChangeMode(this.mode);
  @override
  List<Object?> get props => [mode];
}

class SetMaxSpeed extends ControlEvent {
  final double speedMps;
  const SetMaxSpeed(this.speedMps);
  @override
  List<Object?> get props => [speedMps];
}

class DeployProbes extends ControlEvent {
  const DeployProbes();
}

class RetractProbes extends ControlEvent {
  const RetractProbes();
}

class StartDeseeding extends ControlEvent {
  const StartDeseeding();
}

class StopDeseeding extends ControlEvent {
  const StopDeseeding();
}

// --- States ---

class ControlState extends Equatable {
  final RobotMode mode;
  final double throttle;
  final double steering;
  final double maxSpeed;
  final bool estopActive;
  final bool probesDeployed;
  final bool deseedingActive;

  const ControlState({
    this.mode = RobotMode.idle,
    this.throttle = 0,
    this.steering = 0,
    this.maxSpeed = 1.0,
    this.estopActive = false,
    this.probesDeployed = false,
    this.deseedingActive = false,
  });

  ControlState copyWith({
    RobotMode? mode,
    double? throttle,
    double? steering,
    double? maxSpeed,
    bool? estopActive,
    bool? probesDeployed,
    bool? deseedingActive,
  }) => ControlState(
    mode: mode ?? this.mode,
    throttle: throttle ?? this.throttle,
    steering: steering ?? this.steering,
    maxSpeed: maxSpeed ?? this.maxSpeed,
    estopActive: estopActive ?? this.estopActive,
    probesDeployed: probesDeployed ?? this.probesDeployed,
    deseedingActive: deseedingActive ?? this.deseedingActive,
  );

  @override
  List<Object?> get props => [mode, throttle, steering, maxSpeed, estopActive, probesDeployed, deseedingActive];
}

// --- Bloc ---

class ControlBloc extends Bloc<ControlEvent, ControlState> {
  final RobotRepository _repo;
  Timer? _controlTimer;

  ControlBloc(this._repo) : super(const ControlState()) {
    on<JoystickUpdate>(_onJoystick);
    on<EmergencyStopPressed>(_onEStop);
    on<ChangeMode>(_onChangeMode);
    on<SetMaxSpeed>(_onSetMaxSpeed);
    on<DeployProbes>(_onDeployProbes);
    on<RetractProbes>(_onRetractProbes);
    on<StartDeseeding>(_onStartDeseeding);
    on<StopDeseeding>(_onStopDeseeding);
  }

  void _onJoystick(JoystickUpdate event, Emitter<ControlState> emit) {
    if (state.estopActive) return;

    final scaledThrottle = event.throttle * state.maxSpeed;
    final scaledSteering = event.steering * state.maxSpeed;

    emit(state.copyWith(
      throttle: event.throttle,
      steering: event.steering,
    ));

    // Send at 20Hz rate limiting
    _controlTimer?.cancel();
    _controlTimer = Timer(const Duration(milliseconds: 50), () {
      _repo.sendManualControl(scaledThrottle, scaledSteering);
    });
  }

  Future<void> _onEStop(EmergencyStopPressed event, Emitter<ControlState> emit) async {
    await _repo.sendEmergencyStop();
    emit(state.copyWith(
      estopActive: true,
      throttle: 0,
      steering: 0,
      mode: RobotMode.estop,
      deseedingActive: false,
    ));
  }

  Future<void> _onChangeMode(ChangeMode event, Emitter<ControlState> emit) async {
    if (state.estopActive && event.mode != RobotMode.idle) return;
    await _repo.setMode(event.mode);
    emit(state.copyWith(
      mode: event.mode,
      estopActive: event.mode == RobotMode.estop,
    ));
  }

  void _onSetMaxSpeed(SetMaxSpeed event, Emitter<ControlState> emit) {
    emit(state.copyWith(maxSpeed: event.speedMps.clamp(0.1, 2.0)));
  }

  Future<void> _onDeployProbes(DeployProbes event, Emitter<ControlState> emit) async {
    await _repo.sendCommand(CommandId.deployProbes);
    emit(state.copyWith(probesDeployed: true));
  }

  Future<void> _onRetractProbes(RetractProbes event, Emitter<ControlState> emit) async {
    await _repo.sendCommand(CommandId.retractProbes);
    emit(state.copyWith(probesDeployed: false));
  }

  Future<void> _onStartDeseeding(StartDeseeding event, Emitter<ControlState> emit) async {
    await _repo.sendCommand(CommandId.startDeseeding);
    emit(state.copyWith(deseedingActive: true));
  }

  Future<void> _onStopDeseeding(StopDeseeding event, Emitter<ControlState> emit) async {
    await _repo.sendCommand(CommandId.stopDeseeding);
    emit(state.copyWith(deseedingActive: false));
  }

  @override
  Future<void> close() {
    _controlTimer?.cancel();
    return super.close();
  }
}
