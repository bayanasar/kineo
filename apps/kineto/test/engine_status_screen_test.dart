import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:kineto/demo_workspace.dart';
import 'package:kineto/native/kineto_engine.dart';
import 'package:wabisabi/wabisabi.dart';

void main() {
  testWidgets('demo workspace renders first shot generation and scene progress', (tester) async {
    var requestedShot = -1;
    const shotOne = KinetoDemoSnapshot(
      intent: KinetoDemoIntent.intimacy,
      generated: true,
      selectedIndex: 1,
      locked: false,
      candidateCount: 3,
      generation: 2,
      supersededCount: 3,
    );
    const shotTwo = KinetoDemoSnapshot(
      intent: KinetoDemoIntent.reaction,
      generated: false,
      selectedIndex: null,
      locked: false,
      candidateCount: 0,
      generation: 0,
      supersededCount: 0,
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: WabTheme.materialTheme(lightTheme: false),
        home: DemoWorkspaceView(
          engineSnapshot: const KinetoEngineSnapshot(abiVersion: 8),
          demoSnapshot: shotOne,
          shotSnapshots: const <KinetoDemoSnapshot>[shotOne, shotTwo],
          activeShotIndex: 0,
          onShotChanged: (index) => requestedShot = index,
          onIntentChanged: (_) {},
          onGenerate: () {},
          onSelect: (_) {},
          onLock: () {},
          onReset: () {},
        ),
      ),
    );

    expect(find.text('Scene 001'), findsOneWidget);
    expect(find.text('In progress'), findsOneWidget);
    expect(find.text('Selected'), findsNWidgets(2));
    expect(find.text('Empty'), findsOneWidget);
    expect(find.text('Shot 001'), findsOneWidget);
    expect(find.text('Shot 002'), findsOneWidget);
    expect(find.text('Scene 001 · Shot 001'), findsOneWidget);
    expect(find.text('Wide master'), findsOneWidget);
    expect(find.text('Profile medium'), findsOneWidget);
    expect(find.text('Close-up'), findsOneWidget);
    expect(find.text('Generation 2 · Candidates'), findsOneWidget);
    expect(find.textContaining('3 superseded'), findsOneWidget);
    expect(find.text('Native ABI 8'), findsOneWidget);

    await tester.tap(find.text('Shot 002'));
    expect(requestedShot, 1);
  });

  testWidgets('fully locked scene reports ready while second shot stays independently visible', (tester) async {
    const shotOne = KinetoDemoSnapshot(
      intent: KinetoDemoIntent.reaction,
      generated: true,
      selectedIndex: 0,
      locked: true,
      candidateCount: 3,
      generation: 1,
      supersededCount: 0,
    );
    const shotTwo = KinetoDemoSnapshot(
      intent: KinetoDemoIntent.tension,
      generated: true,
      selectedIndex: 2,
      locked: true,
      candidateCount: 3,
      generation: 1,
      supersededCount: 0,
    );

    await tester.pumpWidget(
      MaterialApp(
        theme: WabTheme.materialTheme(lightTheme: false),
        home: DemoWorkspaceView(
          engineSnapshot: const KinetoEngineSnapshot(abiVersion: 8),
          demoSnapshot: shotTwo,
          shotSnapshots: const <KinetoDemoSnapshot>[shotOne, shotTwo],
          activeShotIndex: 1,
          onShotChanged: (_) {},
          onIntentChanged: (_) {},
          onGenerate: () {},
          onSelect: (_) {},
          onLock: () {},
          onReset: () {},
        ),
      ),
    );

    expect(find.text('Ready'), findsOneWidget);
    expect(find.text('Locked'), findsNWidgets(3));
    expect(find.text('Scene 001 · Shot 002'), findsOneWidget);
    expect(find.text('Doorway two-shot'), findsOneWidget);
    expect(find.text('Over shoulder'), findsOneWidget);
    expect(find.text('Exit detail'), findsOneWidget);
    expect(find.text("Direction is frozen with this shot's locked selection."), findsOneWidget);
    expect(find.text('Selection locked'), findsOneWidget);
    expect(find.text('Reset shot'), findsOneWidget);
  });
}
