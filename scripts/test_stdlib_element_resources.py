"""Check the element-resource model and deliberately invalid clients.

From the repository root, with the pinned Viper tools and Java installed:
    ./x.py exec python3 scripts/test_stdlib_element_resources.py
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
    source = root / "prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/element_resources.vpr"
    logs = root / "target/stdlib-im3-resources-logs"
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
    witness = """
  var a: Ref := new(value)
  var b: Ref := new(value)
  var c: Ref := new(value)
  a.value := 1; b.value := 2; c.value := 3
  fold element(0, a); fold element(1, b)
  var xs: Seq[Ref] := Seq(a, b)
  fold referents(xs, 0)
  two_yields(xs, false)
"""
    negatives = {
        "false": (witness + "  assert false\n", "Assertion false might not hold"),
        "stale": (witness + "  assert at(xs, 0, 0) == 1\n", "Assertion at(xs, 0, 0) == 1 might not hold"),
        "wrong_target": (witness + "  assert at(xs, 0, 1) == 7\n", "Assertion at(xs, 0, 1) == 7 might not hold"),
        "replacement": (witness + """
  var ys: Seq[Ref]
  var previous: Ref
  ys, previous := replace(xs, 0, c)
  assert previous.value == 3
""", "Assertion previous.value == 3 might not hold"),
        "borrowed_owner": (witness + """
  var head: Ref
  var tail: Seq[Ref]
  head, tail := next(xs, 0)
  assert referents(xs, 0)
""", "There might be insufficient permission to access referents(xs, 0)"),
        "duplicated_mutable": ("""
  var a: Ref := new(value)
  fold element(0, a)
  fold element(1, a)
""", "Folding element(1, a) might fail. There might be insufficient permission to access r.value"),
        "shared_write": ("""
  var a: Ref := new(value)
  a.value := 1
  exhale acc(a.value, 1/2)
  a.value := 7
""", "There might be insufficient permission to access a.value"),
        "zero_sized_false": ("""
  var r: Ref
  fold empty_element(0, r)
  fold empty_element(1, r)
  assert false
""", "Assertion false might not hold"),
    }
    cases = [("model", None, None)] + [
        (name, body, expected) for name, (body, expected) in negatives.items()
    ]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", choices=[name for name, _, _ in cases])
    args = parser.parse_args()
    if args.case:
        cases = [case for case in cases if case[0] == args.case]
    passed = True
    with tempfile.TemporaryDirectory(prefix="stdlib-im3-resources-", dir=root / "target") as temp:
        for name, negative, expected in cases:
            program = source.read_text()
            if negative is not None:
                program += "\nmethod negative_witness() {\n" + negative + "}\n"
            path = Path(temp) / (name + ".vpr")
            path.write_text(program)
            args = command + (["--includeMethods", "negative_witness"] if negative else []) + [str(path)]
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
    print(f"All element-resource checks passed ({len(cases)} checks).")


if __name__ == "__main__":
    main()
