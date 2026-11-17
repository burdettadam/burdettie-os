# Plan-only helper applies this to the pinned packages without changing the lock.
# Keep the full LLVM source input. Only compiler-rt needs a writable source copy.
{ packages, nativeLinkerRecipe, stage }:
let
  p = packages;
  identity = package: { drv = package.drvPath; out = package.outPath; };
  paths = inputs: map (input: toString input) inputs;
  patchOnce = text: before: after:
    let removed = builtins.replaceStrings [ before ] [ "" ] text;
    in assert builtins.stringLength text - builtins.stringLength removed
      == builtins.stringLength before;
      builtins.replaceStrings [ before ] [ after ] text;
  linkerPlan = nativeLinkerRecipe p;
  nativeLd = p.native-ld.overrideAttrs (old: {
    buildPhase = patchOnce old.buildPhase
      "ninja -C build-nix-native host_ld host_strip"
      "ninja -j1 -C build-nix-native host_ld host_strip";
  });
  cross = p.darwin-cross-toolchain.override { inherit nativeLd; };
  wrapperNames = input: map (path: builtins.elemAt
    (builtins.match "[a-z0-9]{32}-(.*)" (baseNameOf path)) 0) input.paths;
  changedWrappers = builtins.filter
    (index: builtins.elemAt cross.paths index != builtins.elemAt p.darwin-cross-toolchain.paths index)
    (builtins.genList (index: index) (builtins.length cross.paths));
  rewiredCompilerRt = p.compiler-rt.override { inherit nativeLd; darwinCrossToolchain = cross; };
  compilerRt = rewiredCompilerRt.overrideAttrs (old: {
    unpackPhase = ''
      runHook preUnpack
      test -d "$src/compiler-rt"
      test ! -L "$src/compiler-rt"
      mkdir source
      cp -pr -- "$src/compiler-rt" source/compiler-rt
      chmod -R u+w source/compiler-rt
      for entry in "$src"/* "$src"/.[!.]* "$src"/..?*; do
        if [ ! -e "$entry" ] && [ ! -L "$entry" ]; then continue; fi
        name="$(basename "$entry")"
        if [ "$name" != compiler-rt ]; then ln -s -- "$entry" "source/$name"; fi
      done
      sourceRoot=source
      runHook postUnpack
    '';
    buildPhase = patchOnce old.buildPhase "ninja -C build" "ninja -j1 -C build";
    installPhase = patchOnce old.installPhase "ninja -C build install" "ninja -j1 -C build install";
  });
  rewiredLibsystem = p.libsystem.override {
    inherit nativeLd compilerRt;
    darwinCrossToolchain = cross;
  };
  libsystem = rewiredLibsystem.overrideAttrs (old: {
    buildPhase = patchOnce old.buildPhase "ninja -C build-nix " "ninja -j1 -C build-nix ";
  });
  replaceCross = builtins.replaceStrings [ p.darwin-cross-toolchain.outPath ] [ cross.outPath ];
  replaceLibraries = builtins.replaceStrings
    [ p.native-ld.outPath p.darwin-cross-toolchain.outPath p.compiler-rt.outPath ]
    [ nativeLd.outPath cross.outPath compilerRt.outPath ];
  compilerChecks =
    assert toString compilerRt.src == toString p.compiler-rt.src;
    assert compilerRt.postPatch == p.compiler-rt.postPatch;
    assert compilerRt.configurePhase == replaceCross p.compiler-rt.configurePhase;
    assert paths compilerRt.nativeBuildInputs == paths p.compiler-rt.nativeBuildInputs;
    assert paths (compilerRt.buildInputs or [ ]) == paths (p.compiler-rt.buildInputs or [ ]);
    assert compilerRt.buildPhase == patchOnce p.compiler-rt.buildPhase "ninja -C build" "ninja -j1 -C build";
    assert compilerRt.installPhase == patchOnce p.compiler-rt.installPhase "ninja -C build install" "ninja -j1 -C build install";
    true;
  libsystemChecks =
    assert compilerChecks;
    assert toString libsystem.src == toString p.libsystem.src;
    assert (libsystem.postPatch or "") == (p.libsystem.postPatch or "");
    assert libsystem.configurePhase == replaceLibraries p.libsystem.configurePhase;
    assert paths libsystem.nativeBuildInputs == map replaceCross (paths p.libsystem.nativeBuildInputs);
    assert paths libsystem.buildInputs == paths p.libsystem.buildInputs;
    assert libsystem.installPhase == p.libsystem.installPhase;
    assert libsystem.NIX_DARWIN_TOOLCHAIN_DIR == replaceCross p.libsystem.NIX_DARWIN_TOOLCHAIN_DIR;
    assert libsystem.NIX_DARWIN_HOST == p.libsystem.NIX_DARWIN_HOST;
    assert libsystem.buildPhase == patchOnce p.libsystem.buildPhase "ninja -C build-nix " "ninja -j1 -C build-nix ";
    true;
  selected = builtins.getAttr stage {
    cross-toolchain = cross;
    compiler-rt = assert compilerChecks; compilerRt;
    libsystem = assert libsystemChecks; libsystem;
  };
  original = builtins.getAttr stage {
    cross-toolchain = p.darwin-cross-toolchain;
    compiler-rt = p.compiler-rt;
    libsystem = p.libsystem;
  };
in
assert builtins.elem stage [ "cross-toolchain" "compiler-rt" "libsystem" ];
assert identity nativeLd == linkerPlan.candidate;
assert cross.name == p.darwin-cross-toolchain.name;
assert cross.buildCommand == p.darwin-cross-toolchain.buildCommand;
assert wrapperNames cross == wrapperNames p.darwin-cross-toolchain;
# Only the clang, clang++ and ld wrapper bodies consume nativeLd.
assert changedWrappers == [ 0 1 11 ];
assert original.drvPath != selected.drvPath;
assert original.outPath != selected.outPath;
{
  inherit stage;
  sdk = identity p.apple-sdk;
  native_linker = identity nativeLd;
  upstream = identity original;
  candidate = identity selected;
  cross_toolchain = { upstream = identity p.darwin-cross-toolchain; candidate = identity cross; };
  compiler_rt = if stage == "cross-toolchain" then null else {
    upstream = identity p.compiler-rt;
    candidate = identity compilerRt;
    full_source = {
      out = toString compilerRt.src;
      drv = compilerRt.src.drvPath;
      revision = compilerRt.src.rev or null;
      nar_hash = compilerRt.src.outputHash or null;
      retained_unchanged = true;
    };
    staging = {
      copied = [ "compiler-rt" ];
      symlinked = "all_other_top_level_entries";
      unpack_phase = compilerRt.unpackPhase;
      unpack_phase_sha256 = builtins.hashString "sha256" compilerRt.unpackPhase;
    };
    build_phase_sha256 = builtins.hashString "sha256" compilerRt.buildPhase;
    install_phase_sha256 = builtins.hashString "sha256" compilerRt.installPhase;
  };
  preserved = { sdk = true; full_sources = true; target_flags = true; build_modes = true; };
}
