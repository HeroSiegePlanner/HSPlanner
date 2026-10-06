#!/usr/bin/env python3
"""Decompile named functions or virtual addresses from the verified Hero Siege EXE.

Uses the official Ghidra headless launcher and reuses a fingerprinted project.
Runtime, projects, logs and pseudocode remain under ignored outputs/ghidra/.
The executable is inspected statically, never launched.
"""
import argparse
import bisect
import datetime
import json
import os
from pathlib import Path
import re
import struct
import subprocess

from extract_skill_parameters import Binary


ROOT = Path(__file__).resolve().parents[1]
LOCAL = ROOT / "outputs" / "ghidra"


def export_filename(address, name):
    """Keep filenames portable; the address distinguishes truncated symbols."""
    stem = re.sub(r"[^A-Za-z0-9_.@-]", "_", name)[:180]
    return f"{address:x}_{stem}.c"


def project_arguments(projects, project, executable):
    if (projects / f"{project}.gpr").exists():
        return ["-process"]
    return ["-import", str(executable)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("functions", nargs="+", help="Exact gml_ names or 0x virtual addresses")
    parser.add_argument("--ghidra-home", type=Path,
                        default=LOCAL / "ghidra_12.1.4_PUBLIC")
    parser.add_argument("--java-home", type=Path,
                        default=os.environ.get("GHIDRA_JAVA_HOME",
                            "/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home"))
    parser.add_argument("--timeout", type=int, default=120,
                        help="Decompiler timeout per function, in seconds")
    parser.add_argument("--project-suffix", default="",
                        help="Separate persistent project for concurrent analysis (letters, digits, _ or -)")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    if args.project_suffix and not re.fullmatch(r"[A-Za-z0-9_-]+", args.project_suffix):
        parser.error("--project-suffix accepts only letters, digits, _ or -")
    launcher = args.ghidra_home.resolve() / "support" / "analyzeHeadless"
    if not launcher.is_file():
        parser.error(f"Ghidra is not installed at {args.ghidra_home}; see tools/ghidra/README.md")
    if not (args.java_home / "bin" / "java").is_file():
        parser.error("Java 21 JDK not found; pass --java-home or set GHIDRA_JAVA_HOME")

    binary = Binary(args.executable.resolve())
    by_name = {}
    for address, name in binary.names.items():
        by_name.setdefault(name, []).append(address)
    selected = {}
    for selector in args.functions:
        if selector.startswith("0x"):
            try:
                address = int(selector, 16)
            except ValueError:
                parser.error(f"Invalid virtual address: {selector}")
        else:
            matches = by_name.get(selector, [])
            if len(matches) != 1:
                parser.error(f"Expected one match for {selector}, found {len(matches)}; use a 0x address")
            address = matches[0]
        index = bisect.bisect_right(binary.starts, address) - 1
        if index < 0 or address >= binary.base + binary.entries[index][1]:
            parser.error(f"No PE function range contains {selector}")
        start = binary.starts[index]
        end = binary.base + binary.entries[index][1]
        selected[start] = (end, binary.names.get(start, f"FUN_{start:x}"))

    stamp = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%S.%fZ")
    run = LOCAL / "decompiled" / binary.sha256[:16] / stamp
    run.mkdir(parents=True)
    projects = LOCAL / "projects"
    projects.mkdir(parents=True, exist_ok=True)
    project = f"hero-siege-{binary.sha256[:16]}"
    if args.project_suffix:
        project += f"-{args.project_suffix}"
    symbols = run / "symbols.tsv"
    symbols.write_text("".join(f"{address:x}\t{name}\n"
                               for address, name in sorted(binary.names.items())))
    identifiers = []
    for address in range(0x1505AD008, 0x1507FAA00, 8):
        offset = binary.offset(address - 8)
        if offset is None:
            continue
        name = binary.string(struct.unpack_from("<Q", binary.data, offset)[0])
        if re.fullmatch(r"[A-Za-z_][A-Za-z_0-9]*", name):
            identifiers.append(f"{address:x}\tyy_id_{name}\n")
    (run / "identifiers.tsv").write_text("".join(identifiers))
    targets = run / "targets.tsv"
    targets.write_text("".join(f"{start:x}\t{end:x}\t{name}\t{export_filename(start, name)}\n"
                               for start, (end, name) in selected.items()))
    metadata = {
        "executable": str(binary.path), "sha256": binary.sha256,
        "image_base": hex(binary.base), "ghidra_home": str(args.ghidra_home.resolve()),
        "analysis": "Targeted disassembly and decompilation; no whole-program auto-analysis",
        "functions": [{"address": hex(start), "end_exclusive": hex(end), "name": name,
                       "export_file": export_filename(start, name)}
                      for start, (end, name) in selected.items()],
    }
    (run / "source.json").write_text(json.dumps(metadata, indent=2) + "\n")
    command = [str(launcher), str(projects), project]
    command += project_arguments(projects, project, binary.path)
    command += ["-noanalysis", "-max-cpu", "2",
                "-scriptPath", str(ROOT / "tools" / "ghidra"),
                "-postScript", "DecompileHeroSiege.java", str(run), str(args.timeout),
                "-log", str(run / "application.log"),
                "-scriptlog", str(run / "script.log")]
    environment = os.environ.copy()
    environment["JAVA_HOME"] = str(args.java_home.resolve())
    environment["XDG_CONFIG_HOME"] = str(LOCAL / "settings")
    environment["XDG_CACHE_HOME"] = str(LOCAL / "cache")
    print(f"Ghidra project: {projects / project}\nOutput: {run}", flush=True)
    subprocess.run(command, env=environment, check=True)
    missing = [export_filename(start, name) for start, (_, name) in selected.items()
               if not (run / export_filename(start, name)).is_file()]
    if missing or not (run / "complete.txt").is_file():
        raise RuntimeError(f"Incomplete decompilation; inspect {run / 'application.log'}: {missing}")
    print(f"Decompiled {len(selected)} function(s) into {run}")


if __name__ == "__main__":
    main()
