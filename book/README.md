# Loading Darwin — Technical Manuscript

This directory contains draft chapters about the engineering behind Burdettie-OS:
reading Mach-O files, resolving their dependencies, reproducing runtime behavior,
and connecting that runtime to a desktop interface.

The manuscript describes the project's development at several earlier stages.
Code listings, measurements, and compatibility claims may differ from the current
implementation. Use the [project README](../README.md) for setup and the current
source for implementation details.

## Read the chapters

| Chapter | Topic |
| --- | --- |
| [1. The Binary](05-drafting/ch01_the_binary.md) | Mach-O structure and parsing. |
| [2. The Map](05-drafting/ch02_the_map.md) | Mapping segments and preparing memory. |
| [3. The Links](05-drafting/ch03_the_links.md) | Dependencies, symbols, and fixups. |
| [4. The Runtime](05-drafting/ch04_the_runtime.md) | Objective-C metadata and dispatch. |
| [5. The Foundation](05-drafting/ch05_the_foundation.md) | Framework compatibility and shims. |
| [6. The Bridge](05-drafting/ch06_the_bridge.md) | Calling conventions and host boundaries. |
| [7. The Interpreter](05-drafting/ch07_the_interpreter.md) | ARM64 instruction execution. |
| [8. The System](05-drafting/ch08_the_system.md) | System interfaces and runtime infrastructure. |
| [9. The Tools](05-drafting/ch09_the_tools.md) | Tool experiments and validation. |
| [10. The GUI](05-drafting/ch10_the_gui.md) | Rendering, views, controls, and applications. |
| [Appendix A: Stack Map](05-drafting/appendix_a_stack_map.md) | Relationships between platform layers. |
| [Appendix B: Investigation Notes](05-drafting/appendix_b_war_stories.md) | Debugging observations from compatibility work. |

## Writing and structure

The numbered planning directories use a narrative-writing structure for technical
nonfiction. They contain outlines, teaching goals, and drafting notes. See
[how to use this directory](00-start-here/how-to-use-this-book.md) for that workflow.

Examples that refer to externally obtained application or system binaries are
historical experiments. Those binaries are separate local validation inputs;
the [source and artifact policy](../docs/PUBLICATION.md) describes what belongs in
the public project.
