# Applied to the exact pinned flake's packages.x86_64-linux by the plan helper.
# Change one build command; retain SDK, sources, dependencies and build mode.
p:
let
  upstream = p.native-ld;
  sdk = p.apple-sdk;
  before = "ninja -C build-nix-native host_ld host_strip";
  after = "ninja -j1 -C build-nix-native host_ld host_strip";
  removed = builtins.replaceStrings [ before ] [ "" ] upstream.buildPhase;
  occursOnce = builtins.stringLength upstream.buildPhase - builtins.stringLength removed
    == builtins.stringLength before;
  candidate = upstream.overrideAttrs (old: {
    buildPhase = assert occursOnce;
      builtins.replaceStrings [ before ] [ after ] old.buildPhase;
  });
  identity = package: { drv = package.drvPath; out = package.outPath; };
  paths = packages: map (package: package.outPath) packages;
  sdkRoot = "${sdk.outPath}/Platforms/MacOSX.platform/Developer/SDKs/MacOSX.sdk";
in
assert upstream.drvPath != candidate.drvPath;
assert upstream.outPath != candidate.outPath;
assert upstream.configurePhase == candidate.configurePhase;
assert upstream.installPhase == candidate.installPhase;
assert toString upstream.src == toString candidate.src;
assert paths upstream.nativeBuildInputs == paths candidate.nativeBuildInputs;
assert paths upstream.buildInputs == paths candidate.buildInputs;
assert builtins.replaceStrings [ sdkRoot ] [ "" ] candidate.configurePhase != candidate.configurePhase;
{
  upstream = identity upstream;
  candidate = identity candidate;
  sdk = identity sdk;
  patch_sha256 = builtins.hashString "sha256" (before + "\n" + after + "\n");
  upstream_build_phase_sha256 = builtins.hashString "sha256" upstream.buildPhase;
  candidate_build_phase_sha256 = builtins.hashString "sha256" candidate.buildPhase;
  unchanged = { source = true; dependencies = true; configure = true; install = true; };
}
