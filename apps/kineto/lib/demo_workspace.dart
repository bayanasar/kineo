import 'package:flutter/material.dart';
import 'package:wabisabi/wabisabi.dart';

import 'native/kineto_engine.dart';

final class DemoCandidateContent {
  const DemoCandidateContent({
    required this.title,
    required this.description,
    required this.icon,
  });

  final String title;
  final String description;
  final IconData icon;
}

final class DemoShotContent {
  const DemoShotContent({
    required this.label,
    required this.setting,
    required this.description,
    required this.candidates,
  });

  final String label;
  final String setting;
  final String description;
  final List<DemoCandidateContent> candidates;
}

const List<DemoShotContent> demoShots = <DemoShotContent>[
  DemoShotContent(
    label: 'Shot 001',
    setting: 'Interior · Night',
    description:
        'A filmmaker waits across the table. The final line lands, and the reaction matters more than the dialogue.',
    candidates: <DemoCandidateContent>[
      DemoCandidateContent(
        title: 'Wide master',
        description: '35 mm · static frame · establishes the room and both performers.',
        icon: Icons.panorama_wide_angle,
      ),
      DemoCandidateContent(
        title: 'Profile medium',
        description: '50 mm · slow dolly in · keeps the exchange intimate but spatially clear.',
        icon: Icons.person_outline,
      ),
      DemoCandidateContent(
        title: 'Close-up',
        description: '85 mm · locked frame · prioritizes the final reaction and eye line.',
        icon: Icons.face_outlined,
      ),
    ],
  ),
  DemoShotContent(
    label: 'Shot 002',
    setting: 'Interior · Night · Doorway',
    description:
        'The filmmaker stands to leave. The second beat needs to preserve geography while carrying the tension into the doorway.',
    candidates: <DemoCandidateContent>[
      DemoCandidateContent(
        title: 'Doorway two-shot',
        description: '40 mm · restrained pan · keeps both performers and the exit in one readable frame.',
        icon: Icons.sensor_door_outlined,
      ),
      DemoCandidateContent(
        title: 'Over shoulder',
        description: '65 mm · shoulder foreground · compresses distance as the conversation breaks.',
        icon: Icons.switch_account_outlined,
      ),
      DemoCandidateContent(
        title: 'Exit detail',
        description: '90 mm · static insert · isolates the hand on the door before the cut.',
        icon: Icons.pan_tool_alt_outlined,
      ),
    ],
  ),
];

class DemoWorkspaceScreen extends StatefulWidget {
  const DemoWorkspaceScreen({required this.engine, super.key});

  final KinetoEngine engine;

  @override
  State<DemoWorkspaceScreen> createState() => _DemoWorkspaceScreenState();
}

class _DemoWorkspaceScreenState extends State<DemoWorkspaceScreen> {
  late final KinetoEngineSnapshot _engineSnapshot;
  late List<KinetoDemoSnapshot> _shotSnapshots;
  int _activeShotIndex = 0;
  String? _error;

  @override
  void initState() {
    super.initState();
    _engineSnapshot = widget.engine.snapshot;
    _shotSnapshots = _readShotSnapshots();
  }

  List<KinetoDemoSnapshot> _readShotSnapshots() => List<KinetoDemoSnapshot>.generate(
        KinetoEngine.demoShotCount,
        widget.engine.demoSnapshot,
        growable: false,
      );

  void _switchShot(int index) {
    setState(() {
      _activeShotIndex = index;
      _error = null;
    });
  }

  void _run(void Function() action) {
    try {
      action();
      setState(() {
        _shotSnapshots = _readShotSnapshots();
        _error = null;
      });
    } on Object catch (error) {
      setState(() {
        _shotSnapshots = _readShotSnapshots();
        _error = error.toString();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final demoSnapshot = _shotSnapshots[_activeShotIndex];
    return DemoWorkspaceView(
      engineSnapshot: _engineSnapshot,
      demoSnapshot: demoSnapshot,
      shotSnapshots: _shotSnapshots,
      activeShotIndex: _activeShotIndex,
      error: _error,
      onShotChanged: _switchShot,
      onIntentChanged: (intent) => _run(
        () => widget.engine.setDemoIntent(_activeShotIndex, intent),
      ),
      onGenerate: () => _run(
        () => widget.engine.generateDemoCandidates(_activeShotIndex),
      ),
      onSelect: (candidateIndex) => _run(
        () => widget.engine.selectDemoCandidate(_activeShotIndex, candidateIndex),
      ),
      onLock: () => _run(
        () => widget.engine.lockDemoSelection(_activeShotIndex),
      ),
      onReset: () => _run(
        () => widget.engine.resetDemo(_activeShotIndex),
      ),
    );
  }
}

class DemoWorkspaceView extends StatelessWidget {
  const DemoWorkspaceView({
    required this.engineSnapshot,
    required this.demoSnapshot,
    required this.shotSnapshots,
    required this.activeShotIndex,
    required this.onShotChanged,
    required this.onIntentChanged,
    required this.onGenerate,
    required this.onSelect,
    required this.onLock,
    required this.onReset,
    this.error,
    super.key,
  });

  final KinetoEngineSnapshot engineSnapshot;
  final KinetoDemoSnapshot demoSnapshot;
  final List<KinetoDemoSnapshot> shotSnapshots;
  final int activeShotIndex;
  final ValueChanged<int> onShotChanged;
  final ValueChanged<KinetoDemoIntent> onIntentChanged;
  final VoidCallback onGenerate;
  final ValueChanged<int> onSelect;
  final VoidCallback onLock;
  final VoidCallback onReset;
  final String? error;

  @override
  Widget build(BuildContext context) {
    final selectedIndex = demoSnapshot.selectedIndex;
    final canLock = demoSnapshot.generated && selectedIndex != null && !demoSnapshot.locked;
    final shot = demoShots[activeShotIndex];
    final sceneReady = shotSnapshots.every((snapshot) => snapshot.locked);

    return WabScaffold(
      title: const Text('Kineto'),
      body: SingleChildScrollView(
        padding: const EdgeInsets.all(24),
        child: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 1040),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                WabPanel(
                  title: 'Local demo project',
                  trailing: Text('Native ABI ${engineSnapshot.abiVersion}'),
                  child: const Text(
                    'A deterministic local vertical slice. Work through two independent shots, choose direction, '
                    'generate candidates, select and lock, then reopen Kineto to verify Rust-owned state.',
                  ),
                ),
                const SizedBox(height: 20),
                WabPanel(
                  title: 'Scene 001',
                  trailing: WabStatusBadge(
                    sceneReady ? 'Ready' : 'In progress',
                    kind: sceneReady ? WabBadgeKind.done : WabBadgeKind.progress,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      const Text('Two production beats share the scene but keep independent approval state.'),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 16,
                        runSpacing: 12,
                        children: <Widget>[
                          for (var index = 0; index < demoShots.length; index++)
                            Row(
                              mainAxisSize: MainAxisSize.min,
                              children: <Widget>[
                                WabButton(
                                  kind: index == activeShotIndex
                                      ? WabMaterialKind.seal
                                      : WabMaterialKind.paper,
                                  onPressed: () => onShotChanged(index),
                                  child: Text(demoShots[index].label),
                                ),
                                const SizedBox(width: 8),
                                WabStatusBadge(
                                  _shotStatusLabel(shotSnapshots[index]),
                                  kind: _shotStatusKind(shotSnapshots[index]),
                                ),
                              ],
                            ),
                        ],
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 20),
                WabPanel(
                  title: 'Shot direction',
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        demoSnapshot.locked
                            ? 'Direction is frozen with this shot\'s locked selection.'
                            : 'Changing direction invalidates only this shot\'s unlocked candidates and selection.',
                      ),
                      const SizedBox(height: 12),
                      Wrap(
                        spacing: 12,
                        runSpacing: 12,
                        children: <Widget>[
                          for (final intent in KinetoDemoIntent.values)
                            WabButton(
                              kind: intent == demoSnapshot.intent
                                  ? WabMaterialKind.seal
                                  : WabMaterialKind.paper,
                              onPressed: demoSnapshot.locked
                                  ? null
                                  : () => onIntentChanged(intent),
                              child: Text(intent.label),
                            ),
                        ],
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 20),
                WabPanel(
                  title: 'Scene 001 · ${shot.label}',
                  trailing: Text(demoSnapshot.intent.label),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      Text(
                        shot.setting,
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      const SizedBox(height: 8),
                      Text(shot.description),
                      const SizedBox(height: 16),
                      if (!demoSnapshot.generated)
                        WabElevatedButton(
                          text: const Text('Generate 3 deterministic candidates'),
                          callback: onGenerate,
                        )
                      else
                        Row(
                          children: <Widget>[
                            Expanded(
                              child: Text(
                                demoSnapshot.locked
                                    ? 'Generation ${demoSnapshot.generation} locked · reopen the app to verify persistence.'
                                    : 'Generation ${demoSnapshot.generation} · ${demoSnapshot.supersededCount} superseded · choose the shot you want to keep.',
                              ),
                            ),
                            const SizedBox(width: 16),
                            WabButton(
                              kind: WabMaterialKind.paper,
                              onPressed: demoSnapshot.locked ? null : onGenerate,
                              child: const Text('Regenerate'),
                            ),
                          ],
                        ),
                    ],
                  ),
                ),
                if (demoSnapshot.generated) ...<Widget>[
                  const SizedBox(height: 20),
                  WabPanel(
                    title: 'Generation ${demoSnapshot.generation} · Candidates',
                    child: Wrap(
                      spacing: 16,
                      runSpacing: 16,
                      children: List<Widget>.generate(
                        demoSnapshot.candidateCount,
                        (index) {
                          final content = shot.candidates[index];
                          final selected = selectedIndex == index;
                          return SizedBox(
                            width: 300,
                            child: WabCollectionCard(
                              icon: Icon(content.icon, size: 34),
                              title: content.title,
                              description: content.description,
                              buttonLabel: demoSnapshot.locked
                                  ? (selected ? 'Locked' : 'Candidate')
                                  : (selected ? 'Selected' : 'Select'),
                              highlighted: selected,
                              onPressed: demoSnapshot.locked ? null : () => onSelect(index),
                            ),
                          );
                        },
                      ),
                    ),
                  ),
                  const SizedBox(height: 20),
                  Row(
                    children: <Widget>[
                      Expanded(
                        child: WabButton(
                          kind: WabMaterialKind.seal,
                          onPressed: canLock ? onLock : null,
                          expand: true,
                          child: Text(demoSnapshot.locked ? 'Selection locked' : 'Lock selection'),
                        ),
                      ),
                      const SizedBox(width: 16),
                      Expanded(
                        child: WabButton(
                          kind: WabMaterialKind.paper,
                          onPressed: onReset,
                          expand: true,
                          child: const Text('Reset shot'),
                        ),
                      ),
                    ],
                  ),
                ],
                if (error != null) ...<Widget>[
                  const SizedBox(height: 20),
                  WabPanel(
                    title: 'Native operation failed',
                    child: Text(error!),
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }

  static String _shotStatusLabel(KinetoDemoSnapshot snapshot) {
    if (snapshot.locked) return 'Locked';
    if (snapshot.selectedIndex != null) return 'Selected';
    if (snapshot.generated) return 'Candidates';
    return 'Empty';
  }

  static WabBadgeKind _shotStatusKind(KinetoDemoSnapshot snapshot) {
    if (snapshot.locked) return WabBadgeKind.done;
    if (snapshot.generated) return WabBadgeKind.progress;
    return WabBadgeKind.neutral;
  }
}
