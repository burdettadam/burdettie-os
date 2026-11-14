# Opt-in composition. Keep every existing recipe and its defaults unchanged.
{ packages, libtapiRecipe, stagedLinkerRecipe, nativeLinkerRecipe, librariesRecipe, stager, stage }:
let
  p = packages;
  identity = package: { drv = package.drvPath; out = package.outPath; };
  paths = inputs: map toString inputs;
  nativePlan = stagedLinkerRecipe {
    packages = p;
    inherit libtapiRecipe nativeLinkerRecipe stager;
  };
  matches = builtins.filter (input: (input.pname or "") == "apple-libtapi") p.native-ld.buildInputs;
  originalTapi = assert builtins.length matches == 1; builtins.head matches;
  stagedTapi = originalTapi.overrideAttrs (_: {
    unpackPhase = nativePlan.staged_libtapi.unpack_phase;
  });
  # The unchanged libraries recipe applies the job bounds itself. Rebase its
  # entire unbounded baseline, so its phase comparisons use coherent paths.
  native = p.native-ld.override { libtapi = stagedTapi; };
  cross = p.darwin-cross-toolchain.override { nativeLd = native; };
  rt = p.compiler-rt.override { nativeLd = native; darwinCrossToolchain = cross; };
  system = p.libsystem.override {
    nativeLd = native;
    darwinCrossToolchain = cross;
    compilerRt = rt;
  };
  rebased = p // {
    native-ld = native;
    darwin-cross-toolchain = cross;
    compiler-rt = rt;
    libsystem = system;
  };
  original = builtins.getAttr stage {
    cross-toolchain = p.darwin-cross-toolchain;
    compiler-rt = p.compiler-rt;
    libsystem = p.libsystem;
  };
  unbounded = builtins.getAttr stage {
    cross-toolchain = cross;
    compiler-rt = rt;
    libsystem = system;
  };
  previous = librariesRecipe { packages = p; inherit nativeLinkerRecipe stage; };
  selected = librariesRecipe { packages = rebased; inherit nativeLinkerRecipe stage; };
  replaceCross = builtins.replaceStrings [ p.darwin-cross-toolchain.outPath ] [ cross.outPath ];
  replaceLibraries = builtins.replaceStrings
    [ p.native-ld.outPath p.darwin-cross-toolchain.outPath p.compiler-rt.outPath ]
    [ native.outPath cross.outPath rt.outPath ];
  wrapperNames = package: map (path: builtins.elemAt
    (builtins.match "[a-z0-9]{32}-(.*)" (baseNameOf path)) 0) package.paths;
  changedWrappers = builtins.filter
    (index: builtins.elemAt cross.paths index != builtins.elemAt p.darwin-cross-toolchain.paths index)
    (builtins.genList (index: index) (builtins.length cross.paths));
  crossPreserved = {
    name = cross.name == p.darwin-cross-toolchain.name;
    build_command = cross.buildCommand == p.darwin-cross-toolchain.buildCommand;
    wrapper_names = wrapperNames cross == wrapperNames p.darwin-cross-toolchain;
    only_linker_wrappers_changed = changedWrappers == [ 0 1 11 ];
  };
  rtPreserved = {
    source = toString rt.src == toString p.compiler-rt.src;
    post_patch = rt.postPatch == p.compiler-rt.postPatch;
    configure = rt.configurePhase == replaceCross p.compiler-rt.configurePhase;
    native_build_inputs = paths rt.nativeBuildInputs == paths p.compiler-rt.nativeBuildInputs;
    build_inputs = paths (rt.buildInputs or [ ]) == paths (p.compiler-rt.buildInputs or [ ]);
    build_phase = rt.buildPhase == p.compiler-rt.buildPhase;
    install_phase = rt.installPhase == p.compiler-rt.installPhase;
  };
  systemPreserved = {
    source = toString system.src == toString p.libsystem.src;
    post_patch = (system.postPatch or "") == (p.libsystem.postPatch or "");
    configure = system.configurePhase == replaceLibraries p.libsystem.configurePhase;
    native_build_inputs = paths system.nativeBuildInputs == map replaceCross (paths p.libsystem.nativeBuildInputs);
    build_inputs = paths system.buildInputs == paths p.libsystem.buildInputs;
    build_phase = system.buildPhase == p.libsystem.buildPhase;
    install_phase = system.installPhase == p.libsystem.installPhase;
    toolchain_dir = system.NIX_DARWIN_TOOLCHAIN_DIR == replaceCross p.libsystem.NIX_DARWIN_TOOLCHAIN_DIR;
    host_target = system.NIX_DARWIN_HOST == p.libsystem.NIX_DARWIN_HOST;
  };
  allTrue = fields: builtins.all (value: value) (builtins.attrValues fields);
  # Keep cross-only evaluation independent of the LLVM and libSystem inputs.
  preserved = {
    sdk = selected.sdk == nativePlan.sdk && selected.sdk == previous.sdk;
    native_linker = selected.native_linker == nativePlan.candidate;
    baseline_cross = allTrue crossPreserved;
    baseline_compiler_rt = if stage == "cross-toolchain" then null else allTrue rtPreserved;
    baseline_libsystem = if stage != "libsystem" then null else allTrue systemPreserved;
    selected_recipe = allTrue selected.preserved;
  };
in
assert builtins.elem stage [ "cross-toolchain" "compiler-rt" "libsystem" ];
assert identity originalTapi == nativePlan.staged_libtapi.upstream;
assert identity stagedTapi == nativePlan.staged_libtapi.candidate;
assert identity native == nativePlan.rebound_unbounded;
assert builtins.all (value: value == true || value == null) (builtins.attrValues preserved);
assert identity unbounded == selected.upstream;
assert identity original != identity unbounded;
assert selected.candidate != previous.candidate;
assert selected.candidate != selected.upstream;
{
  inherit stage preserved;
  sdk = selected.sdk;
  upstream = identity original;
  previous_bounded = previous.candidate;
  rebound_unbounded = identity unbounded;
  candidate = selected.candidate;
  native_linker = nativePlan;
  cross_toolchain = selected.cross_toolchain;
  compiler_rt = selected.compiler_rt;
  baseline_checks = {
    cross_toolchain = crossPreserved;
    compiler_rt = if stage == "cross-toolchain" then null else rtPreserved;
    libsystem = if stage != "libsystem" then null else systemPreserved;
  };
}
