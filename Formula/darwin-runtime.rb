class DarwinRuntime < Formula
  desc "ARM64 Mach-O loader with ObjC/Foundation runtime — C-ABI library and tools"
  homepage "https://github.com/example/burdettie-os"
  version "0.1.0"
  license any_of: ["MIT", "Apache-2.0"]

  # For local development, install with:
  #   brew install --build-from-source burdettie-os/tap/darwin-runtime --HEAD
  #
  # When a release tarball exists, uncomment and fill in:
  # url "https://github.com/example/burdettie-os/archive/refs/tags/v0.1.0.tar.gz"
  # sha256 "PLACEHOLDER"

  head "https://github.com/example/burdettie-os.git", branch: "main"

  depends_on "rust" => :build
  depends_on :macos

  def install
    # Build the FFI cdylib (cargo install doesn't support library targets)
    system "cargo", "build", "--release", "-p", "darwin-runtime-ffi"

    # Install binary crates via cargo install
    %w[runtime-shell macho-inspect].each do |crate|
      system "cargo", "install", *std_cargo_args(path: "crates/#{crate}")
    end

    # Install demo apps
    %w[hello-darwin control-gallery foundation-explorer].each do |app|
      system "cargo", "install", *std_cargo_args(path: "apps/#{app}")
    end

    # ── Library ───────────────────────────────────────────────────────
    lib.install "target/release/libdarwin_runtime_ffi.dylib"

    # Fix dylib install name to point to the Homebrew prefix
    MachO::Tools.change_dylib_id(
      lib/"libdarwin_runtime_ffi.dylib",
      "#{lib}/libdarwin_runtime_ffi.dylib",
    )

    # ── Header ────────────────────────────────────────────────────────
    include.install "crates/darwin-runtime-ffi/include/darwin_runtime.h"

    # ── pkg-config ────────────────────────────────────────────────────
    (lib/"pkgconfig").mkpath
    pc_content = (buildpath/"packaging/darwin-runtime.pc.in").read
    pc_content = pc_content.gsub("@PREFIX@", prefix.to_s)
    pc_content = pc_content.gsub("@VERSION@", version.to_s)
    (lib/"pkgconfig/darwin-runtime.pc").write(pc_content)
  end

  test do
    # Verify that the library can be loaded and the version function works
    (testpath/"test.c").write <<~C
      #include <darwin_runtime.h>
      #include <stdio.h>
      #include <stdlib.h>
      int main(void) {
          DarwinVersion v = darwin_runtime_version();
          if (v.major == 0 && v.minor == 0 && v.patch == 0) {
              fprintf(stderr, "Version is all zeros\\n");
              return 1;
          }
          printf("darwin-runtime %u.%u.%u\\n", v.major, v.minor, v.patch);

          uintptr_t count = darwin_objc_class_count();
          printf("ObjC classes registered: %lu\\n", (unsigned long)count);

          return 0;
      }
    C

    system ENV.cc, "test.c",
           "-I#{include}", "-L#{lib}", "-ldarwin_runtime_ffi",
           "-o", "test"
    system "./test"
  end
end
