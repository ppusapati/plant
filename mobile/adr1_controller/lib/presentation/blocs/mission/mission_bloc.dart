import 'dart:typed_data';

import 'package:bloc/bloc.dart';
import 'package:equatable/equatable.dart';
import 'package:uuid/uuid.dart';

import '../../../core/constants/protocol.dart';
import '../../../data/datasources/local_storage.dart';
import '../../../data/models/mission.dart';
import '../../../domain/repositories/robot_repository.dart';

// --- Events ---

sealed class MissionEvent extends Equatable {
  const MissionEvent();
  @override
  List<Object?> get props => [];
}

class LoadMissions extends MissionEvent {
  const LoadMissions();
}

class CreateMission extends MissionEvent {
  final String name;
  final List<Waypoint> waypoints;
  final MissionPattern pattern;
  const CreateMission(this.name, this.waypoints, this.pattern);
  @override
  List<Object?> get props => [name, waypoints, pattern];
}

class DeleteMission extends MissionEvent {
  final String id;
  const DeleteMission(this.id);
  @override
  List<Object?> get props => [id];
}

class UploadMission extends MissionEvent {
  final MissionPlan mission;
  const UploadMission(this.mission);
  @override
  List<Object?> get props => [mission];
}

class PauseMission extends MissionEvent {
  const PauseMission();
}

class ResumeMission extends MissionEvent {
  const ResumeMission();
}

class AbortMission extends MissionEvent {
  const AbortMission();
}

// --- State ---

class MissionState extends Equatable {
  final List<MissionPlan> savedMissions;
  final MissionPlan? activeMission;
  final bool uploading;
  final String? error;

  const MissionState({
    this.savedMissions = const [],
    this.activeMission,
    this.uploading = false,
    this.error,
  });

  MissionState copyWith({
    List<MissionPlan>? savedMissions,
    MissionPlan? activeMission,
    bool? uploading,
    String? error,
  }) => MissionState(
    savedMissions: savedMissions ?? this.savedMissions,
    activeMission: activeMission ?? this.activeMission,
    uploading: uploading ?? this.uploading,
    error: error,
  );

  @override
  List<Object?> get props => [savedMissions, activeMission, uploading, error];
}

// --- Bloc ---

class MissionBloc extends Bloc<MissionEvent, MissionState> {
  final RobotRepository _repo;
  static const _uuid = Uuid();

  MissionBloc(this._repo) : super(const MissionState()) {
    on<LoadMissions>(_onLoad);
    on<CreateMission>(_onCreate);
    on<DeleteMission>(_onDelete);
    on<UploadMission>(_onUpload);
    on<PauseMission>(_onPause);
    on<ResumeMission>(_onResume);
    on<AbortMission>(_onAbort);
  }

  void _onLoad(LoadMissions event, Emitter<MissionState> emit) {
    final missions = LocalStorage.getSavedMissions();
    emit(state.copyWith(savedMissions: missions));
  }

  Future<void> _onCreate(CreateMission event, Emitter<MissionState> emit) async {
    final mission = MissionPlan(
      id: _uuid.v4(),
      name: event.name,
      created: DateTime.now(),
      waypoints: event.waypoints,
      pattern: event.pattern,
    );
    await LocalStorage.saveMission(mission);
    final updated = List<MissionPlan>.from(state.savedMissions)..add(mission);
    emit(state.copyWith(savedMissions: updated));
  }

  Future<void> _onDelete(DeleteMission event, Emitter<MissionState> emit) async {
    await LocalStorage.deleteMission(event.id);
    final updated = state.savedMissions.where((m) => m.id != event.id).toList();
    emit(state.copyWith(savedMissions: updated));
  }

  Future<void> _onUpload(UploadMission event, Emitter<MissionState> emit) async {
    emit(state.copyWith(uploading: true, error: null));
    try {
      await _repo.sendCommand(CommandId.clearWaypoints);
      for (final wp in event.mission.waypoints) {
        final data = _encodeWaypoint(wp);
        await _repo.sendCommand(CommandId.addWaypoint, data);
        await Future.delayed(const Duration(milliseconds: 50));
      }
      await _repo.sendCommand(CommandId.startMission);
      emit(state.copyWith(activeMission: event.mission, uploading: false));
    } catch (e) {
      emit(state.copyWith(uploading: false, error: e.toString()));
    }
  }

  Future<void> _onPause(PauseMission event, Emitter<MissionState> emit) async {
    await _repo.sendCommand(CommandId.pauseMission);
  }

  Future<void> _onResume(ResumeMission event, Emitter<MissionState> emit) async {
    await _repo.sendCommand(CommandId.resumeMission);
  }

  Future<void> _onAbort(AbortMission event, Emitter<MissionState> emit) async {
    await _repo.sendCommand(CommandId.abortMission);
    emit(state.copyWith(activeMission: null));
  }

  /// Encode a waypoint into binary payload for transmission
  static Uint8List _encodeWaypoint(Waypoint wp) {
    final data = ByteData(21);
    data.setFloat64(0, wp.position.latitude, Endian.little);
    data.setFloat64(8, wp.position.longitude, Endian.little);
    data.setUint8(16, wp.action.index);
    data.setFloat32(17, wp.speedMps, Endian.little);
    return data.buffer.asUint8List();
  }
}
