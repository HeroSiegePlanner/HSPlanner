"""Fail-closed regressions for the bounded YYC literal decoder."""
import unittest

from extract_skill_parameters import decode_assignment, nearby_integer, update_xmm_registers


class BinaryStub:
    def constant(self, address):
        return {0x1000: "known ability", 0x2000: 15}.get(address)


class DecodeAssignmentTests(unittest.TestCase):
    def decode(self, instructions, registers=None, previous=None):
        block = [f"{index:x}: {instruction}" for index, instruction in enumerate(instructions, 1)]
        return decode_assignment(BinaryStub(), block, registers or {}, previous or [])

    def assert_unresolved(self, instructions, **kwargs):
        self.assertEqual(self.decode(instructions, **kwargs), (None, None, "unresolved"))

    def test_direct_literals_are_retained(self):
        self.assertEqual(self.decode(["mov edx, 0x2", "call 0x140189f00"])[0], 2)
        self.assertEqual(self.decode(["mov edx, 0xffffffff", "call 0x140189f00"])[0], -1)
        self.assertEqual(self.decode(["movsd xmm1, [rip + 0x10] # 0x2000 ; double: 1.25", "call 0x14018c5a0"])[0], 1.25)
        self.assertEqual(self.decode(["lea rdx, [rip + 0x10] # 0x1000", "call 0x140189e60"])[0], "known ability")

    def test_unknown_integer_writes_discard_old_literal(self):
        for write in ["mov edx, eax", "add edx, eax", "mov dl, al", "xchg eax, edx", "mul ecx"]:
            with self.subTest(write=write):
                self.assert_unresolved(["mov edx, 0x2", write, "call 0x140189f00"])

    def test_weapon_requirement_int64_assignment_preserves_edx_zero_extension(self):
        self.assertEqual(self.decode(["mov edx, 0x2", "call 0x1401cf570"])[0], 2)
        self.assertEqual(self.decode(["mov edx, 0xffffffff", "call 0x1401cf570"])[0], 4294967295)
        for clobber in ["mov rdx, rax", "mov dl, al", "call 0x123", "jne 0x456"]:
            with self.subTest(clobber=clobber):
                self.assert_unresolved(["mov edx, 0x2", clobber, "call 0x1401cf570"])

    def test_unknown_float_writes_discard_old_literal(self):
        for write in ["addsd xmm1, xmm7", "movsd xmm1, qword ptr [rax]", "cvtsi2sd xmm1, eax"]:
            with self.subTest(write=write):
                self.assert_unresolved(["movsd xmm1, xmm6", write, "call 0x14018c5a0"], registers={"xmm6": 2.0, "xmm7": 3.0})

    def test_unknown_copy_pointer_write_discards_old_constant(self):
        self.assert_unresolved(["lea rdx, [rip + 0x10] # 0x1000", "mov rdx, rax", "call 0x140189e60"])

    def test_unrelated_call_clobbers_volatile_arguments(self):
        self.assert_unresolved(["mov edx, 0x2", "call 0x123", "call 0x140189f00"])
        self.assert_unresolved(["movsd xmm1, xmm6", "call 0x123", "call 0x14018c5a0"], registers={"xmm6": 2.0})
        self.assertEqual(self.decode(["call 0x123", "mov edx, 0x4", "call 0x140189f00"])[0], 4)

    def test_callee_saved_float_survives_but_volatile_source_does_not(self):
        self.assertEqual(self.decode(["call 0x123", "movsd xmm1, xmm6", "call 0x14018c5a0"], registers={"xmm6": 2.0})[0], 2.0)
        self.assert_unresolved(["call 0x123", "movsd xmm1, xmm2", "call 0x14018c5a0"], registers={"xmm2": 2.0})

    def test_unknown_source_register_write_invalidates_float_cache(self):
        self.assert_unresolved(["addsd xmm6, xmm7", "movsd xmm1, xmm6", "call 0x14018c5a0"], registers={"xmm6": 2.0, "xmm7": 3.0})
        registers = {"xmm6": 2.0}
        update_xmm_registers("1: movaps xmm6, xmmword ptr [rsp]", registers)
        self.assertNotIn("xmm6", registers)

    def test_control_flow_and_unresolved_first_assignment_fail_closed(self):
        self.assert_unresolved(["mov edx, 0x2", "jne 0x456", "call 0x140189f00"])
        self.assert_unresolved(["call 0x140189f00", "mov edx, 0x2", "call 0x140189f00"])

    def test_reads_do_not_clobber_a_literal(self):
        self.assertEqual(self.decode(["mov edx, 0x2", "cmp edx, eax", "test edx, edx", "call 0x140189f00"])[0], 2)

    def test_nearby_register_write_must_be_recognized(self):
        self.assertEqual(nearby_integer("rdi", ["1: mov edi, 0x5"]), 5)
        for write in ["dec edi", "and edi, 0x1", "mov dil, al", "xchg eax, edi"]:
            with self.subTest(write=write):
                self.assertIsNone(nearby_integer("rdi", ["1: mov edi, 0x5", "2: " + write]))
        self.assertEqual(self.decode(["mov edi, 0x5", "lea edx, [rdi + 0x2]", "call 0x140189f00"])[0], 7)


if __name__ == "__main__":
    unittest.main()
