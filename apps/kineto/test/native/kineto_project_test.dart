import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:kineto/native/kineto_engine.dart';

void main() {
  test('canonical project create and open round-trip through Code Asset', () {
    final sandbox = Directory.systemTemp.createTempSync('kineto-project-ffi-');
    final target = Directory('${sandbox.path}${Platform.pathSeparator}film');

    try {
      final created = KinetoProjectSession.createTextProject(
        path: target.path,
        projectId: 'ffi_project_001',
        title: 'FFI Project',
        createdAt: '2026-09-10T08:00:00Z',
        language: 'en',
        sourceText: 'A filmmaker waits across the table.\n',
      );

      expect(created.formatVersion, 1);
      expect(created.isReadOnly, isFalse);
      expect(created.projectId, 'ffi_project_001');
      expect(created.title, 'FFI Project');
      expect(File('${target.path}${Platform.pathSeparator}project.toml').existsSync(), isTrue);
      expect(
        File(
          '${target.path}${Platform.pathSeparator}source${Platform.pathSeparator}story.txt',
        ).readAsStringSync(),
        'A filmmaker waits across the table.\n',
      );

      created.close();
      expect(() => created.title, throwsStateError);
      created.close();

      final reopened = KinetoProjectSession.open(target.path);
      expect(reopened.formatVersion, 1);
      expect(reopened.isReadOnly, isFalse);
      expect(reopened.projectId, 'ffi_project_001');
      expect(reopened.title, 'FFI Project');
      reopened.close();
    } finally {
      sandbox.deleteSync(recursive: true);
    }
  });

  test('canonical project refuses a created_at the schema would reject', () {
    final sandbox = Directory.systemTemp.createTempSync('kineto-project-ffi-');
    final target = Directory('${sandbox.path}${Platform.pathSeparator}film');

    try {
      expect(
        () => KinetoProjectSession.createTextProject(
          path: target.path,
          projectId: 'ffi_project_003',
          title: 'Bad Timestamp',
          createdAt: 'yesterday',
          language: 'en',
          sourceText: 'source',
        ),
        throwsA(
          isA<KinetoProjectException>().having(
            (error) => error.code,
            'code',
            104,
          ),
        ),
      );
      expect(target.existsSync(), isFalse);
    } finally {
      sandbox.deleteSync(recursive: true);
    }
  });

  test('canonical project create refuses to replace an existing target', () {
    final sandbox = Directory.systemTemp.createTempSync('kineto-project-ffi-');
    final target = Directory('${sandbox.path}${Platform.pathSeparator}film')..createSync();

    try {
      expect(
        () => KinetoProjectSession.createTextProject(
          path: target.path,
          projectId: 'ffi_project_002',
          title: 'Existing',
          createdAt: '2026-09-10T08:00:00Z',
          language: 'en',
          sourceText: 'source',
        ),
        throwsA(
          isA<KinetoProjectException>().having(
            (error) => error.code,
            'code',
            103,
          ),
        ),
      );
    } finally {
      sandbox.deleteSync(recursive: true);
    }
  });
}
