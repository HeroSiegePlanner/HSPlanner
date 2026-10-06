#!/usr/bin/env python3
"""Read-only, conservative GetSubTalentInfo coefficient extractor for this YYC build.
Disassembles the supplied executable without running it. Requires LLVM objdump.
It extracts constant selectors, not runtime mechanic semantics. Unrecognized assignments and
unmatched subtree prefixes remain explicit unknowns. No game JSON is modified.

Example:
    python3 tools/extract_subskill_parameters.py --exe Hero_Siege.exe --output subskill-parameters.json
"""
import argparse, collections, hashlib, json, pathlib, re, struct
from extract_skill_parameters import (Binary, decode_assignment, instruction, READ_ONLY,
                                      INTEGER_ARGUMENT, IMPLICIT_RDX_WRITES)

NUMERIC_SLOTS = {'1e9a8': 'selector2', '1e9b8': 'base1', '1e9c8': 'base2', '1e9d8': 'base3',
                 '1e9e8': 'perRank1', '1e9f8': 'perRank2', '1ea08': 'perRank3', '1ea98': 'positionIndex'}
LABEL_SLOTS = {'1ea18': 'label1', '1ea28': 'label2', '1ea38': 'label3',
               '1ea48': 'suffix1', '1ea58': 'suffix2', '1ea68': 'suffix3'}
SLOT_PATTERN = re.compile(r'lea\s+rcx, \[rsp \+ 0x([0-9a-f]+)\]')
COPY = '0x140189e60'
LOCALIZE = '0x14383eea0'


def address_of(line):
    return '0x' + line.split(':')[0].strip()


def localization_argument_index(block, call_index):
    """Accept only this build's explicit, constant-index argument-array store."""
    if call_index < 7:
        return None
    parsed = [instruction(line) for line in block[call_index - 7:call_index + 1]]
    if parsed[0] != ('mov', ['eax', '0x10']):
        return None
    if parsed[1][0] != 'imul' or parsed[1][1][:2] != ['rax', 'rax']:
        return None
    if len(parsed[1][1]) != 3 or parsed[1][1][2] not in {'0x0', '0x1', '0x2'}:
        return None
    if parsed[2:5] != [('lea', ['rcx', '[rsp + 0x25ca0]']),
                       ('add', ['rcx', 'rax']), ('mov', ['rax', 'rcx'])]:
        return None
    if parsed[6] != ('mov', ['rcx', 'rax']) or parsed[7][0] != 'call':
        return None
    if not parsed[7][1] or parsed[7][1][0].split()[0] != COPY:
        return None
    return int(parsed[1][1][2], 16)


def parse_block(block, binary, strings):
    """Decode proven assignment arguments; unknown writes always shadow known values.

    Localized labels require an explicit argument0 array store, a GetLocalized
    call, and propagation of its return value into RDX. Seeing a tal_* literal
    elsewhere is insufficient. Volatile and partial-register writes invalidate
    tracked arguments. This is a small build-specific decoder, not an emulator.
    """
    values = {}
    unknown = []
    registers = {}
    localization_argument0 = None

    def invalidate(slot, line, reason):
        key = NUMERIC_SLOTS.get(slot, LABEL_SLOTS.get(slot))
        values[key] = {'value': None, 'address': address_of(line), 'status': 'unknown'}
        unknown.append({'slot': slot, 'address': address_of(line), 'reason': reason})

    for index, line in enumerate(block):
        operation, operands = instruction(line)
        if not operation:
            continue
        writes = operands[:2] if operation in {'xchg', 'xadd'} else operands[:1]
        previous_registers = dict(registers)
        if operation not in READ_ONLY:
            implicit_write = operation in IMPLICIT_RDX_WRITES or operation == 'imul' and len(operands) == 1
            for register, aliases in [('rax', {'rax', 'eax', 'ax', 'al', 'ah'}),
                                      ('rdx', INTEGER_ARGUMENT)]:
                if aliases.intersection(writes) or implicit_write:
                    registers.pop(register, None)
            if operation == 'mov' and operands == ['rdx', 'rax']:
                if 'rax' in previous_registers:
                    registers['rdx'] = previous_registers['rax']
            elif operation == 'lea' and operands and operands[0] == 'rdx':
                match = re.search(r'# (0x[0-9a-f]+)', line)
                text = strings.get(match[1], {}).get('text') if match else None
                if text is not None:
                    registers['rdx'] = (text, address_of(line))
            for destination in writes:
                match = re.fullmatch(r'(?:\w+ ptr )?\[rsp \+ (0x[0-9a-f]+)\]', destination)
                if match:
                    offset = int(match[1], 16)
                    if 0x25ca0 <= offset < 0x25cb0:
                        localization_argument0 = None
                    for slot in NUMERIC_SLOTS | LABEL_SLOTS:
                        if int(slot, 16) <= offset < int(slot, 16) + 16:
                            invalidate(slot, line, 'unrecognized inline RValue write')

        match = SLOT_PATTERN.search(line)
        if match and index + 1 < len(block):
            slot = match[1]
            next_operation, next_operands = instruction(block[index + 1])
            helper = next_operands[0].split()[0] if next_operation == 'call' and next_operands else None
            if helper == '0x140189e00':
                pass  # Constructor, followed by explicit metadata initialization.
            elif slot in NUMERIC_SLOTS:
                value, _, _ = decode_assignment(binary, block[max(0, index - 1):index + 2], {}, [])
                if value is None:
                    invalidate(slot, line, 'unresolved numeric assignment argument')
                else:
                    values[NUMERIC_SLOTS[slot]] = {'value': value, 'address': address_of(block[index - 1]),
                                                    'assignCall': address_of(block[index + 1])}
            elif slot in LABEL_SLOTS:
                value = registers.get('rdx') if helper == COPY else None
                if value is None:
                    invalidate(slot, line, 'unresolved string assignment argument')
                else:
                    values[LABEL_SLOTS[slot]] = {'value': value[0], 'address': value[1],
                                                'assignCall': address_of(block[index + 1])}

        if operation == 'call':
            target = operands[0].split()[0] if operands else ''
            result = None
            if target == COPY:
                argument_index = localization_argument_index(block, index)
                if argument_index == 0:
                    localization_argument0 = registers.get('rdx')
                elif argument_index not in {1, 2}:
                    localization_argument0 = None
            elif target == LOCALIZE:
                result = localization_argument0
                localization_argument0 = None
            else:
                localization_argument0 = None
            registers.clear()
            if result is not None:
                registers['rax'] = result
        elif operation.startswith('j') or operation.startswith('loop'):
            registers.clear()
            localization_argument0 = None
    return values, unknown


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--exe', type=pathlib.Path, required=True)
    ap.add_argument('--objdump', default='objdump', help='Path to LLVM objdump')
    ap.add_argument('--data', type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[1] / 'data' / 'skills')
    ap.add_argument('--output', type=pathlib.Path)
    args = ap.parse_args()
    try:
        binary = Binary(args.exe)
    except ValueError as error:
        ap.error(str(error))
    base = binary.base
    matches = [address for address, name in binary.names.items() if name == 'gml_Script_GetSubTalentInfo']
    if len(matches) != 1:
        ap.error('Expected one GetSubTalentInfo function; cannot establish source boundaries.')
    function_address = matches[0]
    def jump_table(address, count):
        offset = binary.offset(address)
        if offset is None:
            raise ValueError('Jump table outside file-backed PE sections: ' + hex(address))
        return [base + struct.unpack_from('<I', binary.data, offset + 4*i)[0] for i in range(count)]
    strings = {hex(address): {'text': text} for address, text in binary.strings.items()}
    lines = binary.disassemble(function_address, args.objdump)
    addr = {int(m[1],16):i for i,line in enumerate(lines) if (m := re.match(r'([0-9a-f]+):', line))}
    prefixes = []
    for i,line in enumerate(lines):
        if '[rsp + 0x1eae8]' in line and i and (m:=re.search(r'# (0x[0-9a-f]+)', lines[i-1])):
            if text := strings.get(m[1],{}).get('text'):
                prefixes.append((i-1,text))
    if not prefixes:
        ap.error('No recognized subtree prefixes; refusing an empty extraction.')
    defaults, default_unknown = parse_block(lines[:prefixes[0][0]], binary, strings)
    normalize = lambda s: re.sub(r'[^a-z0-9]','',s.lower())
    candidates = collections.defaultdict(list)
    source_skills = []
    for path in sorted(args.data.glob('*.json')):
        for skill in json.loads(path.read_text()):
            if not skill.get('subskills'): continue
            source_skills.append(skill)
            for name in {normalize(skill['classId']+skill['name']), normalize(skill['classId']+skill['id'])}:
                candidates[name].append(skill)
    extracted = []; errors = []
    for prefix_num,(line_start,prefix) in enumerate(prefixes):
        line_end = prefixes[prefix_num+1][0] if prefix_num+1<len(prefixes) else len(lines)
        region = lines[line_start:line_end]
        tables = [(i,m) for i,line in enumerate(region) if (m:=re.search(r'mov\s+eax, dword ptr \[rcx \+ 4\*rax \+ (0x[0-9a-f]+)\]',line))]
        if not tables: errors.append({'prefix':prefix,'reason':'no recognized jump table'}); continue
        ti,tm = tables[0]
        if not any('0xd' in l and 'cmp' in l for l in region[max(0,ti-5):ti]):
            errors.append({'prefix':prefix,'reason':'14-case table guard not recognized'}); continue
        table_va = base + int(tm[1],16)
        starts = jump_table(table_va,14)
        ja = next((re.search(r'ja\s+(0x[0-9a-f]+)',l) for l in region[max(0,ti-5):ti] if re.search(r'ja\s+(0x[0-9a-f]+)',l)),None)
        end = int(ja[1],16) if ja else None
        if end is None or starts != sorted(starts) or any(x not in addr for x in starts+[end]):
            errors.append({'prefix':prefix,'reason':'case boundaries unsupported'}); continue
        found = candidates.get(normalize(prefix),[])
        skill = found[0] if len(found)==1 else None
        entry={'prefix':prefix,'prefixAddress':address_of(lines[line_start]),'tableAddress':hex(table_va),
               'classId':skill['classId'] if skill else None,'skillId':skill['id'] if skill else None,
               'mapping':'exact_normalized_class_plus_name_or_id' if skill else 'unknown',
               'gameValidation':'unknown','nodes':[]}
        prelude,pre_unknown = parse_block(region[:ti], binary, strings)
        entry['preludeUnknownAssignments']=pre_unknown
        for position,(start,stop) in enumerate(zip(starts,starts[1:]+[end]),1):
            values, unknown = parse_block(lines[addr[start]:addr[stop]], binary, strings)
            index = values.get('positionIndex',{}).get('value')
            node = next((n for n in skill.get('subskills',[]) if n['positionIndex']==index),None) if skill else None
            scalar = lambda key: values.get(key,prelude.get(key,defaults.get(key)))
            effects=[]
            for selector in range(1,4):
                label=values.get('label'+str(selector))
                if not label and 'perRank'+str(selector) not in values:
                    continue
                if not label:
                    unknown.append({'slot':'label'+str(selector),
                                    'reason':'numeric coefficient without a recovered localization label'})
                    label={'value':None,'status':'unknown'}
                effects.append({'label':label,'base':scalar('base'+str(selector)),
                                'perRank':scalar('perRank'+str(selector)),
                                'baseSelector':5+selector,'perRankSelector':8+selector,
                                'suffix':values.get('suffix'+str(selector)),
                                'runtimeApplication':'unknown'})
            entry['nodes'].append({'positionIndex':index,'tableCase':position-1,'branchStart':hex(start),'branchEndExclusive':hex(stop),
                 'nodeId':node['id'] if node else None,'metadata':node,'selector2':scalar('selector2'),
                 'coefficients':effects,'assignments':values,'unknownAssignments':unknown,
                 'gameValidation':'unknown',
                 'verification':'coefficients_only_runtime_application_unverified'})
        entry['core']={'positionIndex':0,'metadata':next((n for n in skill.get('subskills',[]) if n['positionIndex']==0),None) if skill else None,
                       'verification':'unknown_no_separate_switch_case','defaults':defaults,'prelude':prelude}
        extracted.append(entry)
    report={'schemaVersion':1,'gameValidation':'unknown','binary':{'path':str(args.exe),'sha256':binary.sha256},
            'source':{'function':'gml_Script_GetSubTalentInfo','address':hex(function_address),
                      'disassemblySha256':hashlib.sha256('\n'.join(lines).encode()).hexdigest(),
                      'inputs':'disassembly and strings derived directly from fingerprint-verified executable'},
            'limits':['Static constant coefficients only; matching numbers do not prove a stat is applied to the same damage component.',
                      'Class/name matching is normalization only; no fuzzy matching or descriptions used as runtime evidence.',
                      'Base/perRank selector semantics verified for LoadSubTalentDamage; selector2 semantics not yet interpreted.',
                      'Core node is recorded but lacks a separate node0 switch branch. Unrecognized assignments stay unknown.'],
            'defaults':defaults,'defaultUnknownAssignments':default_unknown,
            'counts':{'prefixes':len(prefixes),'extractedSubtrees':len(extracted),
               'mappedSubtrees':sum(s['skillId'] is not None for s in extracted),'nonCoreNodes':sum(len(s['nodes']) for s in extracted),
               'recordedCores':len(extracted),'dataSubtrees':len(source_skills),'errors':len(errors),
               'unknownAssignments':sum(len(n['unknownAssignments']) for s in extracted for n in s['nodes'])},
            'errors':errors,'subtrees':extracted}
    output=json.dumps(report,indent=2,ensure_ascii=False)+'\n'
    if args.output: args.output.write_text(output)
    else: print(output,end='')
if __name__=='__main__': main()
