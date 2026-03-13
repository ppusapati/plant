import 'dart:async';
import 'package:bloc/bloc.dart';
import 'package:equatable/equatable.dart';

import '../../../core/constants/protocol.dart';
import '../../../domain/repositories/robot_repository.dart';

// --- Events ---

sealed class ConnectionEvent extends Equatable {
  const ConnectionEvent();
  @override
  List<Object?> get props => [];
}

class ConnectWiFi extends ConnectionEvent {
  final String address;
  final int port;
  const ConnectWiFi(this.address, {this.port = 8080});
  @override
  List<Object?> get props => [address, port];
}

class ConnectBle extends ConnectionEvent {
  final String deviceId;
  final String deviceName;
  const ConnectBle(this.deviceId, this.deviceName);
  @override
  List<Object?> get props => [deviceId, deviceName];
}

class Disconnect extends ConnectionEvent {
  const Disconnect();
}

class ConnectionLost extends ConnectionEvent {
  const ConnectionLost();
}

class ReconnectAttempt extends ConnectionEvent {
  const ReconnectAttempt();
}

// --- States ---

sealed class ConnectionState extends Equatable {
  const ConnectionState();
  @override
  List<Object?> get props => [];
}

class ConnectionInitial extends ConnectionState {
  const ConnectionInitial();
}

class Connecting extends ConnectionState {
  final ConnectionType type;
  final String target;
  const Connecting(this.type, this.target);
  @override
  List<Object?> get props => [type, target];
}

class Connected extends ConnectionState {
  final ConnectionType type;
  final String target;
  final int rssi;
  const Connected(this.type, this.target, {this.rssi = 0});
  @override
  List<Object?> get props => [type, target, rssi];
}

class ConnectionFailed extends ConnectionState {
  final String message;
  const ConnectionFailed(this.message);
  @override
  List<Object?> get props => [message];
}

class Disconnected extends ConnectionState {
  const Disconnected();
}

class Reconnecting extends ConnectionState {
  final int attempt;
  final int maxAttempts;
  const Reconnecting(this.attempt, this.maxAttempts);
  @override
  List<Object?> get props => [attempt, maxAttempts];
}

// --- Bloc ---

class ConnectionBloc extends Bloc<ConnectionEvent, ConnectionState> {
  final RobotRepository _repo;
  StreamSubscription? _telemetrySub;
  Timer? _reconnectTimer;
  String? _lastAddress;
  ConnectionType? _lastType;
  int _reconnectAttempts = 0;
  static const _maxReconnectAttempts = 5;

  ConnectionBloc(this._repo) : super(const ConnectionInitial()) {
    on<ConnectWiFi>(_onConnectWiFi);
    on<ConnectBle>(_onConnectBle);
    on<Disconnect>(_onDisconnect);
    on<ConnectionLost>(_onConnectionLost);
    on<ReconnectAttempt>(_onReconnectAttempt);
  }

  Future<void> _onConnectWiFi(ConnectWiFi event, Emitter<ConnectionState> emit) async {
    emit(Connecting(ConnectionType.wifi, event.address));
    try {
      await _repo.connectWiFi(event.address, port: event.port);
      _lastAddress = event.address;
      _lastType = ConnectionType.wifi;
      _reconnectAttempts = 0;
      _watchConnection();
      emit(Connected(ConnectionType.wifi, event.address));
    } catch (e) {
      emit(ConnectionFailed('WiFi: ${e.toString()}'));
    }
  }

  Future<void> _onConnectBle(ConnectBle event, Emitter<ConnectionState> emit) async {
    emit(Connecting(ConnectionType.ble, event.deviceName));
    try {
      await _repo.connectBle(event.deviceId);
      _lastAddress = event.deviceId;
      _lastType = ConnectionType.ble;
      _reconnectAttempts = 0;
      _watchConnection();
      emit(Connected(ConnectionType.ble, event.deviceName));
    } catch (e) {
      emit(ConnectionFailed('BLE: ${e.toString()}'));
    }
  }

  Future<void> _onDisconnect(Disconnect event, Emitter<ConnectionState> emit) async {
    _reconnectTimer?.cancel();
    await _telemetrySub?.cancel();
    await _repo.disconnect();
    _reconnectAttempts = 0;
    emit(const Disconnected());
  }

  void _onConnectionLost(ConnectionLost event, Emitter<ConnectionState> emit) {
    if (_lastAddress != null && _reconnectAttempts < _maxReconnectAttempts) {
      _reconnectAttempts++;
      emit(Reconnecting(_reconnectAttempts, _maxReconnectAttempts));
      _reconnectTimer = Timer(
        Duration(seconds: _reconnectAttempts * 2), // exponential backoff
        () => add(const ReconnectAttempt()),
      );
    } else {
      emit(const Disconnected());
    }
  }

  Future<void> _onReconnectAttempt(ReconnectAttempt event, Emitter<ConnectionState> emit) async {
    if (_lastAddress == null) return;
    try {
      if (_lastType == ConnectionType.wifi) {
        await _repo.connectWiFi(_lastAddress!);
      } else {
        await _repo.connectBle(_lastAddress!);
      }
      _reconnectAttempts = 0;
      _watchConnection();
      emit(Connected(_lastType!, _lastAddress!));
    } catch (_) {
      add(const ConnectionLost());
    }
  }

  void _watchConnection() {
    _telemetrySub?.cancel();
    _telemetrySub = _repo.telemetryStream.listen((telemetry) {
      if (!telemetry.connected && state is Connected) {
        add(const ConnectionLost());
      }
    });
  }

  @override
  Future<void> close() {
    _reconnectTimer?.cancel();
    _telemetrySub?.cancel();
    return super.close();
  }
}
