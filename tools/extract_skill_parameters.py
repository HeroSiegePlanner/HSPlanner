#!/usr/bin/env python3
"""Extract auditable class-skill metadata from the supplied YYC Windows binary.

Static inspection only: the executable is never run. Requires LLVM objdump.
This is deliberately version-pinned: helper addresses are known only for the
fingerprint below. Unknown field assignments remain null. Metadata extraction
is not proof of projectile collision, effective rank, or DPS behavior.

Usage: python3 tools/extract_skill_parameters.py Hero_Siege.exe --output audit.json
"""
import argparse
import bisect
import hashlib
import json
import pathlib
import re
import struct
import subprocess

KNOWN_SHA256 = "9b5b5d068e96914db2de98b482d380ec1b905443c446bff25dd360f8e11398ba"
PREFIX = "gml_Script_PopulateTalentStructMap"


class Binary:
    def __init__(self, path):
        self.path = pathlib.Path(path)
        self.data = self.path.read_bytes()
        self.sha256 = hashlib.sha256(self.data).hexdigest()
        if self.sha256 != KNOWN_SHA256:
            raise ValueError("Unsupported executable fingerprint; revalidate helper addresses before extracting")
        pe = struct.unpack_from("<I", self.data, 60)[0]
        self.base = struct.unpack_from("<Q", self.data, pe + 48)[0]
        self.sections = {}
        for index in range(struct.unpack_from("<H", self.data, pe + 6)[0]):
            at = pe + 24 + struct.unpack_from("<H", self.data, pe + 20)[0] + 40 * index
            name = self.data[at:at + 8].rstrip(b"\0").decode()
            size, rva, raw_size, raw = struct.unpack_from("<IIII", self.data, at + 8)
            self.sections[name] = (self.base + rva, raw_size, raw, size)
        va, _, raw, size = self.sections[".pdata"]
        self.entries = [struct.unpack_from("<III", self.data, at) for at in range(raw, raw + size, 12)]
        self.starts = [self.base + item[0] for item in self.entries]
        self.names = {}
        self.strings = {}
        va, size, raw, _ = self.sections[".text"]
        code = self.data[raw:raw + size]
        for match in re.finditer(rb"[\x48\x4c]\x8d[\x05\x0d\x15\x1d\x25\x2d\x35\x3d]", code):
            at = match.start()
            address = va + at
            target = address + 7 + struct.unpack_from("<i", code, at + 3)[0]
            name = self.string(target)
            if name.startswith("gml_"):
                index = bisect.bisect_right(self.starts, address) - 1
                if index >= 0 and address < self.base + self.entries[index][1]:
                    self.names.setdefault(self.starts[index], name)
        for match in re.finditer(rb"\x48\x8d\x15....\x48\x8d\x0d....\xe8", code, re.S):
            at = match.start()
            address = va + at
            target = address + 7 + struct.unpack_from("<i", code, at + 3)[0]
            global_address = address + 14 + struct.unpack_from("<i", code, at + 10)[0]
            value = self.string(target)
            if value:
                self.strings[global_address] = value

    def offset(self, address):
        for va, size, raw, _ in self.sections.values():
            if va <= address < va + size:
                return raw + address - va
        return None

    def string(self, address):
        offset = self.offset(address)
        if offset is None:
            return ""
        end = self.data.find(b"\0", offset, offset + 500)
        value = self.data[offset:end] if end >= 0 else b""
        return value.decode() if value and all(32 <= byte < 127 for byte in value) else ""

    def constant(self, address):
        if address in self.strings:
            return self.strings[address]
        offset = self.offset(address)
        if offset is None:
            return None
        value, _, kind = struct.unpack_from("<QII", self.data, offset)
        if kind in (0, 13):
            return struct.unpack("<d", struct.pack("<Q", value))[0]
        if kind == 10:
            return struct.unpack("<q", struct.pack("<Q", value))[0]
        return None

    def annotate(self, line):
        match = re.search(r"# (0x[0-9a-f]+)", line)
        if not match:
            return line
        address = int(match[1], 16)
        offset = self.offset(address)
        if offset is None:
            return line
        if 0x1505AD000 <= address < 0x1507FAA00:
            name = self.string(struct.unpack_from("<Q", self.data, offset - 8)[0])
            if name:
                line += " ; field: " + name
        elif "movsd" in line:
            line += " ; double: " + str(struct.unpack_from("<d", self.data, offset)[0])
        return line

    def disassemble(self, address, objdump):
        index = bisect.bisect_left(self.starts, address)
        end = self.base + self.entries[index][1]
        result = subprocess.run([objdump, "-d", "--no-show-raw-insn", "--x86-asm-syntax=intel",
                                 "--start-address=" + hex(address), "--stop-address=" + hex(end),
                                 str(self.path)], text=True, capture_output=True, check=True)
        return [self.annotate(line) for line in result.stdout.splitlines()]


def instruction(line):
    """Parse objdump's instruction and retain annotations for literal loads."""
    match = re.match(r"\s*[0-9a-f]+:\s*([a-z][a-z0-9]*)\s*(.*)", line)
    if not match:
        return None, []
    operation, operands = match.groups()
    operands = re.split(r"[#;]", operands, maxsplit=1)[0].strip()
    return operation, [value.strip() for value in operands.split(",")] if operands else []


READ_ONLY = {"cmp", "test", "push", "comisd", "ucomisd", "comiss", "ucomiss"}
INTEGER_ARGUMENT = {"rdx", "edx", "dx", "dl", "dh"}
IMPLICIT_RDX_WRITES = {"mul", "div", "idiv", "cqo", "cdq", "cwd", "cpuid", "rdtsc", "rdtscp", "xgetbv"}


def update_xmm_registers(line, registers):
    operation, operands = instruction(line)
    if operation == "call":
        for index in range(6):
            registers.pop("xmm" + str(index), None)
        return
    if not operands or operation in READ_ONLY:
        return
    destination = operands[0]
    if not re.fullmatch(r"xmm\d+", destination):
        return
    value = None
    if operation == "movsd" and (match := re.search(r"double: (.+)", line)):
        value = float(match[1])
    elif operation in {"movsd", "movaps", "movapd"} and len(operands) == 2:
        value = registers.get(operands[1])
    if value is None:
        registers.pop(destination, None)
    else:
        registers[destination] = value


def nearby_integer(register, previous):
    aliases = {"rdi": {"edi", "rdi", "di", "dil"},
               "rsi": {"esi", "rsi", "si", "sil"},
               "rbx": {"ebx", "rbx", "bx", "bl", "bh"}}
    for line in reversed(previous):
        operation, operands = instruction(line)
        if not operands or operation in READ_ONLY:
            continue
        writes = operands[:2] if operation in {"xchg", "xadd"} else operands[:1]
        if not aliases[register].intersection(writes):
            continue
        destination = operands[0]
        source = operands[1] if len(operands) > 1 else ""
        if operation == "xor" and source == destination and destination in {register, "e" + register[1:]}:
            return 0
        if operation == "mov" and destination in {register, "e" + register[1:]} and re.fullmatch(r"-?0x[0-9a-f]+", source):
            return int(source, 0)
        return None
    return None


def decode_assignment(binary, block, registers, previous):
    integer = copied = None
    registers = dict(registers)
    helpers = {"0x140189f00": "integer assignment",
               "0x1401cf570": "zero-extended integer assignment",
               "0x14018c5a0": "double assignment",
               "0x140189e60": "constant RValue copy"}
    for index, line in enumerate(block):
        operation, operands = instruction(line)
        if operation == "call":
            target = operands[0].split()[0] if operands else ""
            if target in helpers:
                value = {"0x140189f00": integer,
                         "0x1401cf570": integer & 0xffffffff if integer is not None else None,
                         "0x14018c5a0": registers.get("xmm1"),
                         "0x140189e60": copied}[target]
                if value is not None:
                    return value, "0x" + line.split(":")[0].strip(), helpers[target]
                return None, None, "unresolved"
            integer = copied = None
        if operation and (operation.startswith("j") or operation.startswith("loop")):
            return None, None, "unresolved"
        writes = operands[:2] if operation in {"xchg", "xadd"} else operands[:1]
        implicit_write = operation in IMPLICIT_RDX_WRITES or operation == "imul" and len(operands) == 1
        if implicit_write or (operation not in READ_ONLY and INTEGER_ARGUMENT.intersection(writes)):
            integer = copied = None
            if match := re.search(r"mov\s+edx, (-?0x[0-9a-f]+)\b", line):
                integer = int(match[1], 0)
            elif re.search(r"xor\s+edx, edx\b", line):
                integer = 0
            elif match := re.search(r"lea\s+edx, \[(rdi|rsi|rbx) ([+-]) (0x[0-9a-f]+)\]", line):
                value = nearby_integer(match[1], previous + block[:index])
                integer = value + int(match[3], 16) * (1 if match[2] == "+" else -1) if value is not None else None
            elif match := re.search(r"lea\s+rdx, \[rip .*# (0x[0-9a-f]+)", line):
                copied = binary.constant(int(match[1], 16))
            if integer is not None:
                integer = (integer + 2**31) % 2**32 - 2**31
        update_xmm_registers(line, registers)
    return None, None, "unresolved"


def extract(binary, objdump):
    skills = []
    for address, name in sorted(binary.names.items()):
        if not name.startswith(PREFIX) or name == PREFIX:
            continue
        lines = binary.disassemble(address, objdump)
        fields = [(index, match[1]) for index, line in enumerate(lines)
                  if (match := re.search(r"field: ((?:ability|proc|glyph)\w+)", line))]
        registers = {}
        register_cursor = 0
        groups = []
        group = []
        case_start = 0
        for position, (index, key) in enumerate(fields):
            for line in lines[register_cursor:index]:
                update_xmm_registers(line, registers)
            register_cursor = index
            if key == "abilityLevelReq":
                if group:
                    groups.append(group)
                group = []
                case_start = index
            end = fields[position + 1][0] if position + 1 < len(fields) else len(lines)
            value, assignment, method = decode_assignment(binary, lines[index:end], registers, lines[case_start:index])
            group.append({"field": key, "value": value, "source": "0x" + lines[index].split(":")[0],
                          "assignment": assignment, "method": method})
        if group:
            groups.append(group)
        for group in groups:
            identity = next((field for field in group if field["field"] == "abilityId"), None)
            if identity:
                skills.append({"class_function": name, "function_address": hex(address),
                               "abilityId": identity["value"], "fields": group})
    identities = {(skill["class_function"], skill["abilityId"]) for skill in skills}
    if len(skills) != 432 or len(identities) != 432 or any(skill["abilityId"] is None for skill in skills):
        raise ValueError("Extraction coverage changed: expected 432 identified class skills")
    return skills


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=pathlib.Path)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--objdump", default="objdump")
    args = parser.parse_args()
    binary = Binary(args.executable)
    skills = extract(binary, args.objdump)
    document = {"sha256": binary.sha256, "executable_size": len(binary.data), "image_base": hex(binary.base),
                "limitations": ["Metadata only; collision and DPS behavior are separate code paths.",
                                "Missing assignments are null; constructor defaults are not inferred.",
                                "StartingValue and Value are raw fields; rank offsets and runtime multipliers are not inferred."],
                "skills": skills}
    args.output.write_text(json.dumps(document, indent=2, allow_nan=False) + "\n")
    print(f"Extracted {len(skills)} identified class skills to {args.output}")


if __name__ == "__main__":
    main()
