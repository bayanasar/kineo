import 'dart:convert';
import 'dart:ffi';

import 'package:ffi/ffi.dart';

part 'kineto_project.dart';

typedef _DestroyEngineNative = Void Function(Pointer<Void>);

@Native<Pointer<Void> Function()>(symbol: 'kineto_engine_create')
external Pointer<Void> _kinetoEngineCreate();

@Native<_DestroyEngineNative>(symbol: 'kineto_engine_destroy')
external void _kinetoEngineDestroy(Pointer<Void> engine);

@Native<Uint32 Function()>(
  symbol: 'kineto_engine_abi_version',
  isLeaf: true,
)
external int _kinetoEngineAbiVersion();

@Native<Uint64 Function(Pointer<Void>, Uint32)>(
  symbol: 'kineto_demo_state',
  isLeaf: true,
)
external int _kinetoDemoState(Pointer<Void> engine, int shotIndex);

@Native<Int32 Function(Pointer<Void>, Uint32, Uint32)>(symbol: 'kineto_demo_set_intent')
external int _kinetoDemoSetIntent(Pointer<Void> engine, int shotIndex, int intent);

@Native<Int32 Function(Pointer<Void>, Uint32)>(symbol: 'kineto_demo_generate')
external int _kinetoDemoGenerate(Pointer<Void> engine, int shotIndex);

@Native<Int32 Function(Pointer<Void>, Uint32, Uint32)>(symbol: 'kineto_demo_select')
external int _kinetoDemoSelect(Pointer<Void> engine, int shotIndex, int candidateIndex);

@Native<Int32 Function(Pointer<Void>, Uint32)>(symbol: 'kineto_demo_lock')
external int _kinetoDemoLock(Pointer<Void> engine, int shotIndex);

@Native<Int32 Function(Pointer<Void>, Uint32)>(symbol: 'kineto_demo_reset')
external int _kinetoDemoReset(Pointer<Void> engine, int shotIndex);

/// Native ABI revision these Dart bindings were written against.
///
/// The bundled Rust library and this file are versioned together. Every packed
/// snapshot layout, error code, and exported symbol below belongs to exactly
/// this revision, so a library reporting any other number cannot be decoded
/// safely and must be refused rather than misread.
const int kinetoExpectedNativeAbiVersion = 8;

/// Thrown when the loaded native library does not speak the ABI these bindings
/// were compiled against.
final class KinetoAbiMismatchException implements Exception {
  const KinetoAbiMismatchException(this.expected, this.actual);

  final int expected;
  final int actual;

  @override
  String toString() =>
      'KinetoAbiMismatchException: bindings expect native ABI $expected, '
      'loaded library reports $actual';
}

void _assertAbiCompatible() {
  final actual = _kinetoEngineAbiVersion();
  if (actual != kinetoExpectedNativeAbiVersion) {
    throw KinetoAbiMismatchException(kinetoExpectedNativeAbiVersion, actual);
  }
}

final class KinetoEngineSnapshot {
  const KinetoEngineSnapshot({required this.abiVersion});

  final int abiVersion;
}

enum KinetoDemoIntent {
  reaction(1, 'Reaction'),
  spatialClarity(2, 'Spatial clarity'),
  intimacy(3, 'Intimacy'),
  tension(4, 'Tension');

  const KinetoDemoIntent(this.wireValue, this.label);

  factory KinetoDemoIntent.fromWire(int value) => switch (value) {
        1 => reaction,
        2 => spatialClarity,
        3 => intimacy,
        4 => tension,
        _ => reaction,
      };

  final int wireValue;
  final String label;
}

final class KinetoDemoSnapshot {
  const KinetoDemoSnapshot({
    required this.intent,
    required this.generated,
    required this.selectedIndex,
    required this.locked,
    required this.candidateCount,
    required this.generation,
    required this.supersededCount,
  });

  factory KinetoDemoSnapshot.fromBits(int bits) {
    final selectedCode = (bits >> 16) & 0xff;
    return KinetoDemoSnapshot(
      intent: KinetoDemoIntent.fromWire((bits >> 24) & 0xff),
      generated: (bits & 1) != 0,
      locked: ((bits >> 1) & 1) != 0,
      candidateCount: (bits >> 8) & 0xff,
      selectedIndex: selectedCode == 0 ? null : selectedCode - 1,
      generation: (bits >> 32) & 0xffff,
      supersededCount: (bits >> 48) & 0xffff,
    );
  }

  final KinetoDemoIntent intent;
  final bool generated;
  final int? selectedIndex;
  final bool locked;
  final int candidateCount;
  final int generation;
  final int supersededCount;
}

final class KinetoNativeException implements Exception {
  const KinetoNativeException(this.code, this.message);

  final int code;
  final String message;

  @override
  String toString() => 'KinetoNativeException($code): $message';
}

/// Thin owner for the in-process Rust engine.
///
/// No JSON, RPC dispatcher, localhost socket, child process, or dynamic library
/// path is involved. Flutter resolves the Rust CodeAsset through the library URI
/// and calls the exported ABI directly.
final class KinetoEngine implements Finalizable {
  KinetoEngine._(this._handle) {
    _finalizer.attach(this, _handle, detach: this);
  }

  static const int demoShotCount = 2;

  static final NativeFinalizer _finalizer = NativeFinalizer(
    Native.addressOf<NativeFunction<_DestroyEngineNative>>(_kinetoEngineDestroy),
  );

  final Pointer<Void> _handle;
  bool _closed = false;

  factory KinetoEngine.open() {
    _assertAbiCompatible();
    final handle = _kinetoEngineCreate();
    if (handle == nullptr) {
      throw StateError('Failed to initialize the Kineto Rust engine');
    }
    return KinetoEngine._(handle);
  }

  KinetoEngineSnapshot get snapshot {
    _ensureOpen();
    return KinetoEngineSnapshot(abiVersion: _kinetoEngineAbiVersion());
  }

  KinetoDemoSnapshot demoSnapshot(int shotIndex) {
    _ensureOpen();
    _checkShotIndex(shotIndex);
    return KinetoDemoSnapshot.fromBits(_kinetoDemoState(_handle, shotIndex));
  }

  void setDemoIntent(int shotIndex, KinetoDemoIntent intent) {
    _ensureOpen();
    _checkShotIndex(shotIndex);
    _checkDemoResult(_kinetoDemoSetIntent(_handle, shotIndex, intent.wireValue));
  }

  void generateDemoCandidates(int shotIndex) {
    _ensureOpen();
    _checkShotIndex(shotIndex);
    _checkDemoResult(_kinetoDemoGenerate(_handle, shotIndex));
  }

  void selectDemoCandidate(int shotIndex, int candidateIndex) {
    _ensureOpen();
    _checkShotIndex(shotIndex);
    if (candidateIndex < 0) {
      throw RangeError.value(candidateIndex, 'candidateIndex', 'Candidate index must be non-negative');
    }
    _checkDemoResult(_kinetoDemoSelect(_handle, shotIndex, candidateIndex));
  }

  void lockDemoSelection(int shotIndex) {
    _ensureOpen();
    _checkShotIndex(shotIndex);
    _checkDemoResult(_kinetoDemoLock(_handle, shotIndex));
  }

  void resetDemo(int shotIndex) {
    _ensureOpen();
    _checkShotIndex(shotIndex);
    _checkDemoResult(_kinetoDemoReset(_handle, shotIndex));
  }

  void close() {
    if (_closed) return;
    _closed = true;
    _finalizer.detach(this);
    _kinetoEngineDestroy(_handle);
  }

  void _checkShotIndex(int shotIndex) {
    if (shotIndex < 0 || shotIndex >= demoShotCount) {
      throw RangeError.range(shotIndex, 0, demoShotCount - 1, 'shotIndex');
    }
  }

  void _checkDemoResult(int code) {
    if (code == 0) return;
    throw KinetoNativeException(code, _demoErrorMessage(code));
  }

  String _demoErrorMessage(int code) => switch (code) {
        1 => 'Native engine handle is invalid',
        2 => 'Could not persist local demo state',
        3 => 'Local demo state is invalid',
        4 => 'Generate candidates before selecting or locking',
        5 => 'Candidate index is invalid',
        6 => 'Selection is locked; reset the shot to start over',
        7 => 'Select a candidate before locking',
        8 => 'Artifact lifecycle rejected the operation',
        9 => 'Shot direction is invalid',
        10 => 'Shot index is invalid',
        99 => 'Native call failed unexpectedly',
        _ => 'Unknown native demo error',
      };

  void _ensureOpen() {
    if (_closed) {
      throw StateError('Kineto Rust engine is closed');
    }
  }
}
