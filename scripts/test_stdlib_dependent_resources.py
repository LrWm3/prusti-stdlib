"""Check the dependent-resource model and deliberately invalid clients.

From the repository root, with the pinned Viper tools and Java installed:
    ./x.py exec python3 scripts/test_stdlib_dependent_resources.py
"""

import argparse
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


def main():
    root = Path(__file__).resolve().parent.parent
    source = root / "prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/dependent_resources.vpr"
    logs = root / "target/stdlib-im4-dependent-logs"
    logs.mkdir(parents=True, exist_ok=True)
    java_home = os.environ.get("JAVA_HOME")
    java = str(Path(java_home) / "bin/java") if java_home else "java"
    env = os.environ.copy()
    env["Z3_EXE"] = str(root / "viper_tools/z3/bin/z3")
    command = [
        java, "-Xss512m", "-cp", str(root / "viper_tools/backends/viperserver.jar"),
        "viper.silicon.SiliconRunner", "--logLevel", "ERROR", "--enableMoreCompleteExhale",
        "--disableTerminationPlugin", "--assertTimeout", "10000", "--proverConfigArgs",
        "smt.qi.eager_threshold=1000 model.partial=false smt.arith.solver=6",
        "--numberOfParallelVerifiers", "1", "--timeout", "180",
    ]
    original = source.read_text()
    outer_apply = next(line for line in original.splitlines() if line.startswith("  apply slot"))
    frame_a = " && (unfolding pair(a,b) in read(a)) == old[lhs](unfolding pair(a,b) in read(a))"
    frame_b = " && (unfolding pair(a,b) in read(b)) == old[lhs](unfolding pair(a,b) in read(b))"
    cases = [
        ("model", original, None),
        ("false_after_restore", original.replace("  assert group(first) && group(fresh) && group(second)", "  assert false"), "Assertion false might not hold"),
        ("missing_outer", original.replace(outer_apply, ""), "There might be insufficient permission to access pair(a, b)"),
        ("missing_inner", original.replace("  apply (group(first)", "  exhale group(first)\n  apply (group(first)"), "There might be insufficient permission to access group(first)"),
        ("missing_value_frame", original.replace(frame_a, "").replace(frame_b, ""), "Assertion (unfolding acc(pair(a, b), write) in read(a) == fresh) might not hold"),
        ("wrong_replacement", original.replace("  assert unfolding pair(a,b) in read(a) == fresh", "  assert unfolding pair(a,b) in read(a) == Num(99)"), "Assertion (unfolding acc(pair(a, b), write) in read(a) == Num(99)) might not hold"),
    ]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", choices=[name for name, _, _ in cases])
    args = parser.parse_args()
    if args.case:
        cases = [case for case in cases if case[0] == args.case]
    passed = True
    with tempfile.TemporaryDirectory(prefix="stdlib-im4-dependent-", dir=root / "target") as temp:
        for name, program, expected in cases:
            path = Path(temp) / (name + ".vpr")
            path.write_text(program)
            args = command + (["--includeMethods", "caller"] if expected else []) + [str(path)]
            print(f"Running {name} ...", flush=True)
            start = time.monotonic()
            try:
                result = subprocess.run(
                    args, cwd=root, env=env, text=True, stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT, timeout=240,
                )
                output = result.stdout
                errors = re.findall(r"^\s*\[\d+\] (.+)$", output, re.MULTILINE)
                if expected is None:
                    ok = result.returncode == 0 and "finished verification successfully" in output and not errors
                else:
                    ok = result.returncode == 1 and len(errors) == 1 and expected in errors[0]
                ok = ok and not re.search(r"Verification timed out|internal error|Exception|Parsing error", output)
            except subprocess.TimeoutExpired as error:
                output = error.stdout or b""
                if isinstance(output, bytes):
                    output = output.decode(errors="replace")
                output += "\nTest command exceeded 240 seconds.\n"
                ok = False
            (logs / (name + ".log")).write_text(output)
            print(f"{name}: {'PASS' if ok else 'FAIL'} ({time.monotonic() - start:.2f}s)", flush=True)
            if not ok:
                print(output, flush=True)
            passed &= ok
    if not passed:
        raise SystemExit(1)
    print(f"All dependent-resource checks passed ({len(cases)} checks).")


if __name__ == "__main__":
    main()
