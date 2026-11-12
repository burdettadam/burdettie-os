# Runtime corpus

Each directory contains an authored Objective-C program and, where needed,
metadata for the compatibility runner. Compiled `app` files and `.dSYM`
debug bundles are generated locally and excluded from version control.

Rebuild every `main.m` program on macOS with the Xcode command-line tools:

```sh
./corpus/build_corpus.sh
```

The script discovers source directories in a stable order, targets ARM64,
and stops if compilation fails. A new `main.m` program is included without
adding its name to a separate build list. Building the corpus does not run
its programs; some programs intentionally exercise files, networking or
subprocesses when executed.

Use the resulting programs as local inputs to the runtime and compatibility
tools. Preserve their source and metadata when adding or changing a test.
