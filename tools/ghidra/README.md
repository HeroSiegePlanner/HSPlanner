# Hero Siege decompilation

Use the official Ghidra headless analyzer through the repository helper:

```sh
python3 tools/decompile_hero_siege.py /Users/zium/Downloads/Hero_Siege.exe \
  gml_Script_LoadMonsterBreaks \
  gml_Object_Butcher_Blender_Nanoblades_obj_Create_0
```

A virtual address is also accepted, including an address inside a PE function:

```sh
python3 tools/decompile_hero_siege.py /Users/zium/Downloads/Hero_Siege.exe 0x144243070
```

The helper reuses `extract_skill_parameters.Binary` to verify the executable
SHA-256 and recover GML names and PE function ranges. It labels recovered GML
addresses and disassembles only the requested function ranges before exporting
C pseudocode. It does not run the game. Types, signatures and variable names in
decompiler output are inferred; verify relevant instructions before changing
planner formulas. Large functions can require a higher `--timeout` (seconds).
Full program auto-analysis is disabled, so cross-references and caller/callee
type information can be incomplete.

Each run writes fresh `.c` files, `source.json`, symbol/range inputs, logs and a
completion marker beneath `outputs/ghidra/decompiled/<fingerprint>/<timestamp>/`.
Export filenames begin with the function address and use a bounded symbol name;
`source.json` preserves each full symbol name and its `export_file` mapping.
The persistent project is `outputs/ghidra/projects/hero-siege-<fingerprint>`.
The helper reopens its single root program regardless of the executable's
current filename, so renamed identical copies reuse existing imports. Projects
containing multiple root files are rejected before export; use a separate
`--project-suffix` instead of adding programs to a helper-owned project.
Run one process at a time per project; Ghidra locks open projects. All installation
and generated artifacts remain in the already-ignored `outputs/` directory.
For parallel independent analyses, pass a distinct `--project-suffix` to each
process; each suffix has its own persistent import. YYC identifier cells are
labeled from the verified executable's name table. Large class dispatchers use
a two-million-instruction limit and a 256 MiB response limit; their per-function
time limit still comes from `--timeout`.

## Local installation

- [Official Ghidra 12.1.4 release](https://github.com/NationalSecurityAgency/ghidra/releases/tag/Ghidra_12.1.4_build)
- Archive: `ghidra_12.1.4_PUBLIC_20260921.zip`
- Verified SHA-256: `ddac49f903da9d5bac833e5cc79395098b9c33cfd3279be5f31bd00387d2d4db`
- Extracted runtime: `outputs/ghidra/ghidra_12.1.4_PUBLIC`
- CLI entry point: `support/analyzeHeadless` within that runtime
- JDK: the existing Homebrew OpenJDK 21 installation

The helper sets Java and Ghidra's settings/cache locations for its child process.
It does not change the shell profile or system Java. For a different installation,
pass `--ghidra-home` and `--java-home` (or set `GHIDRA_JAVA_HOME`). Its default
Java path is `/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home`.

To reproduce the Apple Silicon decompiler build after extracting and verifying
the official archive, run these commands from the repository root:

```sh
make -C outputs/ghidra/ghidra_12.1.4_PUBLIC/Ghidra/Features/Decompiler/src/decompile/cpp \
  -j4 ghidra_opt ARCH_TYPE='-arch arm64' \
  ADDITIONAL_FLAGS='-mmacosx-version-min=11.0' OSDIR=mac_arm_64
mkdir -p outputs/ghidra/ghidra_12.1.4_PUBLIC/Ghidra/Features/Decompiler/os/mac_arm_64
cp outputs/ghidra/ghidra_12.1.4_PUBLIC/Ghidra/Features/Decompiler/src/decompile/cpp/ghidra_opt \
  outputs/ghidra/ghidra_12.1.4_PUBLIC/Ghidra/Features/Decompiler/os/mac_arm_64/decompile
```

This builds the native component needed by the headless decompiler using the
installed Xcode command-line tools. The full native toolset has a separate
`support/gradle/gradlew buildNatives` workflow described in Ghidra's
`docs/GettingStarted.md`.

The supported game executable has SHA-256
`9b5b5d068e96914db2de98b482d380ec1b905443c446bff25dd360f8e11398ba`.
Revalidate recovered names and function ranges before supporting another build.

## Verification

Validated on 2026-10-01 with the supplied executable and Ghidra 12.1.4:

- Fresh import exported `gml_Script_LoadMonsterBreaks` and
  `gml_Object_Butcher_Blender_Nanoblades_obj_Create_0` successfully.
- A second run reopened the saved project and resolved the interior address
  `0x1473f207c` to the Nanoblades Create function at `0x1473f1d60`.
- The second export was byte-identical to the first. These checks establish
  working decompilation and project reuse; they do not validate game formulas.
- A renamed identical executable reopened the existing `physical-item` project
  and exported `0x140189fd0` plus the 293-character symbol at `0x1401965f0`.
  Its bounded export filename succeeds while the full symbol remains in metadata.
- A fresh independent `cli-verify` project exported
  `gml_Script_LoadProjectileMeleeSettings` at `0x144298df0`. Its pseudocode
  matched the preceding `butcher-mania` export byte for byte; both SHA-256
  hashes are `80ba0bdfe46c0cb0dbe6f53a34ac61de1381c432f98cffee21aa773418f4ad2e`.

Cheap helper regressions can be run without Ghidra or the executable:

```sh
python3 -m unittest discover -s tools -p test_decompile_hero_siege.py -v
```
