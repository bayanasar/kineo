import 'package:flutter_test/flutter_test.dart';
import 'package:kineto/native/kineto_engine.dart';

void main() {
  test('bindings and bundled native library agree on the ABI revision', () {
    // The guard inside KinetoEngine.open() only helps if this constant is
    // moved whenever the Rust ABI moves. Fail loudly here if they drift.
    final engine = KinetoEngine.open();
    addTearDown(engine.close);
    expect(engine.snapshot.abiVersion, kinetoExpectedNativeAbiVersion);
  });

  test('Flutter loads the bundled Rust CodeAsset through typed FFI', () {
    final engine = KinetoEngine.open();
    addTearDown(engine.close);

    final snapshot = engine.snapshot;
    expect(snapshot.abiVersion, kinetoExpectedNativeAbiVersion);
  });

  test('explicit close is idempotent and native calls reject use-after-close', () {
    final engine = KinetoEngine.open();
    engine.close();
    engine.close();

    expect(() => engine.snapshot, throwsStateError);
    expect(() => engine.demoSnapshot(0), throwsStateError);
    expect(
      () => engine.setDemoIntent(0, KinetoDemoIntent.tension),
      throwsStateError,
    );
  });

  test('shot index is validated before entering native code', () {
    final engine = KinetoEngine.open();
    addTearDown(engine.close);

    expect(() => engine.demoSnapshot(-1), throwsRangeError);
    expect(() => engine.demoSnapshot(KinetoEngine.demoShotCount), throwsRangeError);
  });

  test('two shots keep independent approval state and survive reopen', () {
    var engine = KinetoEngine.open();
    engine.resetDemo(0);
    engine.resetDemo(1);

    engine.setDemoIntent(0, KinetoDemoIntent.tension);
    engine.generateDemoCandidates(0);
    engine.selectDemoCandidate(0, 2);
    engine.lockDemoSelection(0);

    engine.setDemoIntent(1, KinetoDemoIntent.intimacy);
    engine.generateDemoCandidates(1);
    engine.selectDemoCandidate(1, 1);
    engine.generateDemoCandidates(1);
    var shotTwo = engine.demoSnapshot(1);
    expect(shotTwo.generation, 2);
    expect(shotTwo.supersededCount, 3);
    expect(shotTwo.selectedIndex, isNull);
    engine.selectDemoCandidate(1, 0);
    engine.lockDemoSelection(1);

    final shotOneBeforeClose = engine.demoSnapshot(0);
    expect(shotOneBeforeClose.intent, KinetoDemoIntent.tension);
    expect(shotOneBeforeClose.generation, 1);
    expect(shotOneBeforeClose.selectedIndex, 2);
    expect(shotOneBeforeClose.locked, isTrue);

    engine.close();
    engine = KinetoEngine.open();
    try {
      final shotOne = engine.demoSnapshot(0);
      shotTwo = engine.demoSnapshot(1);

      expect(shotOne.intent, KinetoDemoIntent.tension);
      expect(shotOne.generation, 1);
      expect(shotOne.selectedIndex, 2);
      expect(shotOne.locked, isTrue);
      expect(shotOne.supersededCount, 0);

      expect(shotTwo.intent, KinetoDemoIntent.intimacy);
      expect(shotTwo.generation, 2);
      expect(shotTwo.selectedIndex, 0);
      expect(shotTwo.locked, isTrue);
      expect(shotTwo.supersededCount, 3);
    } finally {
      engine.resetDemo(0);
      engine.resetDemo(1);
      engine.close();
    }
  });
}
