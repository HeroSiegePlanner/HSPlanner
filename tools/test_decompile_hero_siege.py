"""Cheap regressions for export paths and reuse of fingerprinted projects."""
from pathlib import Path
import tempfile
import unittest

from decompile_hero_siege import export_filename, project_arguments


class DecompileHelperTests(unittest.TestCase):
    def test_existing_short_export_names_are_preserved(self):
        self.assertEqual(
            export_filename(0x144222E40, "gml_Script_LoadManaCost"),
            "144222e40_gml_Script_LoadManaCost.c",
        )

    def test_long_symbols_export_with_distinct_bounded_names(self):
        name = "gml_Script_" + "a" * 283
        first = export_filename(0x1401965F0, name)
        second = export_filename(0x140197760, name)
        self.assertLessEqual(len(first.encode("utf-8")), 255)
        self.assertNotEqual(first, second)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory) / first
            output.write_text("pseudocode")
            self.assertTrue(output.is_file())

    def test_export_name_cannot_introduce_path_components(self):
        name = export_filename(0x140001000, "gml_../folder\\function")
        self.assertEqual(Path(name).name, name)
        self.assertNotIn("\\", name)

    def test_renamed_identical_executable_reuses_existing_program(self):
        with tempfile.TemporaryDirectory() as directory:
            projects = Path(directory)
            project = "hero-siege-fingerprint"
            original = Path("/first/Hero_Siege.exe")
            renamed = Path("/second/Renamed.exe")
            self.assertEqual(project_arguments(projects, project, original),
                             ["-import", str(original)])
            (projects / f"{project}.gpr").touch()
            for executable in [original, renamed]:
                self.assertEqual(project_arguments(projects, project, executable), ["-process"])


if __name__ == "__main__":
    unittest.main()
