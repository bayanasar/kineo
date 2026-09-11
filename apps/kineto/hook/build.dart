import 'dart:io';

import 'package:hooks/hooks.dart';
import 'package:native_toolchain_rust/native_toolchain_rust.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    // RustBuilder currently discovers Rust source files, but Cargo/toolchain
    // configuration is also an input to the produced Code Asset. Declare it
    // explicitly so the Dart hooks cache cannot reuse a native library built
    // from stale manifests, lock data, or toolchain settings.
    output.dependencies.addAll(<Uri>[
      File('../../Cargo.toml').absolute.uri,
      File('../../Cargo.lock').absolute.uri,
      File('../../rust-toolchain.toml').absolute.uri,
      File('../../crates/kineto-core/Cargo.toml').absolute.uri,
      File('../../crates/kineto-project/Cargo.toml').absolute.uri,
      File('rust/Cargo.toml').absolute.uri,
      File('rust/rust-toolchain.toml').absolute.uri,
    ]);

    await RustBuilder(
      // Match lib/native/kineto_engine.dart so @Native resolves this CodeAsset
      // by its library URI with no platform-specific library lookup code.
      assetName: 'native/kineto_engine.dart',
    ).run(input: input, output: output);
  });
}
