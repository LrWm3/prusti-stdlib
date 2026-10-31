import copy
import importlib.util
from pathlib import Path
import tempfile
import subprocess
import unittest


spec = importlib.util.spec_from_file_location("matrix", Path(__file__).with_name("test_stdlib_matrix.py"))
matrix = importlib.util.module_from_spec(spec)
spec.loader.exec_module(matrix)


class MatrixCoverage(unittest.TestCase):
    def test_source_manifest_includes_new_regression_sources(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            bins = root / "prusti-tests/tests/cargo_verify/stdlib_extern_spec/src/bin"
            bins.mkdir(parents=True)
            source = bins / "new_pass.rs"
            source.write_text("fn main() {}\n")
            hashes = matrix.source_manifest(root, ["extern"])
            self.assertEqual(hashes[source.relative_to(root).as_posix()], matrix.digest(source))

    def test_partition_keeps_every_client_once_and_one_library_per_backend(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            bins = root / "prusti-tests/tests/cargo_verify/stdlib_iterator_spec/src/bin"
            bins.mkdir(parents=True)
            ordinary_names = {"ordinary_pass", "ordinary_wrong", "mut_direct_pass"}
            resource_names = {"mut_native_resource_early_pass",
                              "mut_native_resource_pair_prefix_bad_state",
                              "mut_native_resource_framing_pass", "mut_generic_array_pass",
                              "mut_generic_array_state_wrong", "mut_sequence_state_reborrow_false",
                              "mut_element_nested_pass", "mut_native_mut_ref_restoration_wrong",
                              "mut_native_restoration_wrong_suffix"}
            names = ordinary_names | resource_names
            for name in names:
                (bins / (name + ".rs")).write_text("")
            jobs = matrix.plan(root, ["iterator"], ["Ferrum", "Silicon"])
            for backend in ("Ferrum", "Silicon"):
                selected = [job for job in jobs if job["backend"] == backend]
                actual = [case for job in selected for case in job["cases"]]
                self.assertEqual(set(actual), names)
                self.assertEqual(len(actual), len(names))
                self.assertEqual(sum(job["library"] for job in selected), 1)
                self.assertEqual([job["timeout"] for job in selected], [180, 600])
                self.assertEqual(set(selected[0]["cases"]), ordinary_names)
                self.assertEqual(set(selected[1]["cases"]), resource_names)
                self.assertTrue(selected[1]["library"])

            for name in resource_names:
                (bins / (name + ".rs")).unlink()
            ordinary = matrix.plan(root, ["iterator"], ["Ferrum"])
            self.assertEqual(len(ordinary), 1)
            self.assertTrue(ordinary[0]["library"], "a reduced suite must still verify its library")

    def test_resume_rejects_missing_duplicate_changed_or_wrong_build_results(self):
        job = {"suite": "vec", "backend": "Ferrum", "cases": ["pass", "wrong"],
               "library": True, "timeout": 180}
        hashes = {f"prusti-tests/tests/cargo_verify/stdlib_vec_spec/src/bin/{name}.rs": name
                  for name in job["cases"]}
        data = {"suite": "vec", "backend": "Ferrum", "verifier_timeout": 180,
                "solver_timeout_ms": 30000, "solver_check_timeout_ms": 100,
                "solver_time_bounds": True, "parallel_verifiers": 1,
                "parallelize_branches": False, "spec_library_selected": True,
                "artifacts": {"prusti-driver": {"sha256": "driver"}},
                "cases": [{"case": name, "passed": True, "source_sha256": name}
                          for name in matrix.expected_checks(job)]}
        self.assertTrue(matrix.assess(data, job, hashes, "driver")["complete"])
        broken = copy.deepcopy(data)
        broken["cases"].pop()
        self.assertFalse(matrix.assess(broken, job, hashes, "driver")["complete"])
        broken = copy.deepcopy(data)
        broken["cases"].append(broken["cases"][-1])
        self.assertFalse(matrix.assess(broken, job, hashes, "driver")["complete"])
        self.assertFalse(matrix.assess(data, job, hashes, "different-driver")["complete"])
        changed = dict(hashes)
        changed[next(iter(changed))] = "changed-source"
        self.assertFalse(matrix.assess(data, job, changed, "driver")["complete"])
        self.assertFalse(matrix.assess(data, job, {}, "driver")["complete"])
        broken = copy.deepcopy(data)
        broken["cases"][-1]["passed"] = False
        report = matrix.assess(broken, job, hashes, "driver")
        self.assertTrue(report["complete"], "a finished failure must retain coverage")
        self.assertEqual(len(report["failures"]), 1, "a finished failure must never become a pass")


if __name__ == "__main__":
    unittest.main()
