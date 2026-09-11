import 'package:flutter/material.dart';
import 'package:wabisabi/wabisabi.dart';

import 'demo_workspace.dart';
import 'native/kineto_engine.dart';

void main() {
  WidgetsFlutterBinding.ensureInitialized();
  runApp(KinetoApp(engine: KinetoEngine.open()));
}

class KinetoApp extends StatefulWidget {
  const KinetoApp({required this.engine, super.key});

  final KinetoEngine engine;

  @override
  State<KinetoApp> createState() => _KinetoAppState();
}

class _KinetoAppState extends State<KinetoApp> {
  @override
  void dispose() {
    widget.engine.close();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Kineto',
      theme: WabTheme.materialTheme(lightTheme: false),
      home: DemoWorkspaceScreen(engine: widget.engine),
    );
  }
}
