"""Fail-closed regressions for literal and localization argument recovery."""
import unittest

from extract_subskill_parameters import parse_block


class FakeBinary:
    def constant(self, address):
        return None


def assembly(*instructions):
    return [f'{0x143900000 + index * 16:x}: {text}' for index, text in enumerate(instructions)]


def argument(index, source='lea rdx, [rip + 0x0] # 0x150000010'):
    return [
        'mov eax, 0x10', f'imul rax, rax, 0x{index:x}',
        'lea rcx, [rsp + 0x25ca0]', 'add rcx, rax', 'mov rax, rcx', source,
        'mov rcx, rax', 'call 0x140189e60',
    ]


class SubskillArgumentTests(unittest.TestCase):
    def parse(self, *instructions):
        return parse_block(assembly(*instructions), FakeBinary(), {
            '0x150000010': {'text': 'tal_aoe'},
            '0x150000020': {'text': '%'},
        })

    def test_integer_argument(self):
        values, _ = self.parse('mov edx, 0x6', 'lea rcx, [rsp + 0x1e9e8]', 'call 0x140189f00')
        self.assertEqual(values['perRank1']['value'], 6)

    def test_double_argument_requires_xmm1(self):
        for register, expected in [('xmm1', 17.5), ('xmm0', None)]:
            with self.subTest(register=register):
                values, _ = self.parse(f'movsd {register}, qword ptr [rip + 0x0] # 0x14f000000 ; double: 17.5',
                                       'lea rcx, [rsp + 0x1e9e8]', 'call 0x14018c5a0')
                self.assertEqual(values['perRank1']['value'], expected)

    def test_unknown_numeric_write_shadows_known_zero(self):
        values, unknown = self.parse('xor edx, edx', 'lea rcx, [rsp + 0x1e9e8]', 'call 0x140189f00',
                                    'mov edx, eax', 'lea rcx, [rsp + 0x1e9e8]', 'call 0x140189f00')
        self.assertIsNone(values['perRank1']['value'])
        self.assertEqual(values['perRank1']['status'], 'unknown')
        self.assertTrue(unknown)

    def test_localized_label_tracks_actual_argument0(self):
        values, _ = self.parse(*argument(0), *argument(1, 'mov rdx, rbx'),
                              *argument(2, 'mov rdx, rbx'), 'call 0x14383eea0', 'mov rdx, rax',
                              'lea rcx, [rsp + 0x1ea18]', 'call 0x140189e60')
        self.assertEqual(values['label1']['value'], 'tal_aoe')

    def test_literal_does_not_leak_into_later_label(self):
        values, _ = self.parse('lea rdx, [rip + 0x0] # 0x150000010',
                              'lea rcx, [rsp + 0x1ea18]', 'call 0x140189e60',
                              'mov rdx, rax', 'lea rcx, [rsp + 0x1ea28]', 'call 0x140189e60')
        self.assertEqual(values['label1']['value'], 'tal_aoe')
        self.assertIsNone(values['label2']['value'])

    def test_unknown_string_write_shadows_label_and_suffix(self):
        for slot, key in [('1ea18', 'label1'), ('1ea48', 'suffix1')]:
            with self.subTest(slot=slot):
                values, _ = self.parse('lea rdx, [rip + 0x0] # 0x150000020',
                                      f'lea rcx, [rsp + 0x{slot}]', 'call 0x140189e60',
                                      'mov rdx, rbx', f'lea rcx, [rsp + 0x{slot}]', 'call 0x140189e60')
                self.assertIsNone(values[key]['value'])
                self.assertEqual(values[key]['status'], 'unknown')

    def test_partial_register_write_invalidates_string(self):
        values, _ = self.parse('lea rdx, [rip + 0x0] # 0x150000010', 'mov dl, 0x1',
                              'lea rcx, [rsp + 0x1ea18]', 'call 0x140189e60')
        self.assertIsNone(values['label1']['value'])

    def test_unknown_call_clobbers_localized_return(self):
        values, _ = self.parse(*argument(0), 'call 0x14383eea0', 'call 0x140000001',
                              'mov rdx, rax', 'lea rcx, [rsp + 0x1ea18]', 'call 0x140189e60')
        self.assertIsNone(values['label1']['value'])

    def test_localization_argument_not_reused_after_call(self):
        values, _ = self.parse(*argument(0), 'call 0x14383eea0', 'call 0x14383eea0',
                              'mov rdx, rax', 'lea rcx, [rsp + 0x1ea18]', 'call 0x140189e60')
        self.assertIsNone(values['label1']['value'])

    def test_unknown_argument0_replaces_known_argument0(self):
        values, _ = self.parse(*argument(0), *argument(0, 'mov rdx, rbx'), 'call 0x14383eea0',
                              'mov rdx, rax', 'lea rcx, [rsp + 0x1ea18]', 'call 0x140189e60')
        self.assertIsNone(values['label1']['value'])

    def test_inline_partial_value_write_invalidates_known_numeric(self):
        values, _ = self.parse('mov edx, 0x6', 'lea rcx, [rsp + 0x1e9e8]', 'call 0x140189f00',
                              'mov dword ptr [rsp + 0x1e9f4], eax')
        self.assertIsNone(values['perRank1']['value'])

    def test_branch_invalidates_string_argument(self):
        values, _ = self.parse('lea rdx, [rip + 0x0] # 0x150000010', 'jne 0x143900040',
                              'lea rcx, [rsp + 0x1ea18]', 'call 0x140189e60')
        self.assertIsNone(values['label1']['value'])


if __name__ == '__main__':
    unittest.main()
