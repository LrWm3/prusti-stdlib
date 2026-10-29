"""Check the generic-resource model and deliberately invalid clients.

From the repository root, with the pinned Viper tools and Java installed:
    ./x.py exec python3 scripts/test_stdlib_generic_resources.py
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
    source = root / "prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/generic_resources.vpr"
    logs = root / "target/stdlib-im4-groups-logs"
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
  var r: Ref := new(value)
  r.value := Num(7)
  fold physical(r, IntType())
  fold indexed_physical(0, r)
  var xs: Seq[Ref] := Seq(r)
  pack(xs)
  unpack(xs)
"""
    negatives = {
        "conversion_false": (witness + "  assert false\n", "Assertion false might not hold"),
        "conversion_wrong_value": (witness + "  assert physical_value_at(0, r) == 8\n", "Assertion physical_value_at(0, r) == 8 might not hold"),
        "equal_empty_false": ("  two_equal_owned_values()\n  assert false\n", "Assertion false might not hold"),
        "forge_unknown": ("  fold group(UnknownSnap(0), UnknownType(0))\n", "There might be insufficient permission to access opaque_group(s, t)"),
        "missing_referent": ("  var r: Ref\n  fold group(Ptr(r), RefType(IntType()))\n", "There might be insufficient permission to access physical(get_Snap_address(s), get_Type_inner(t))"),
        "duplicate_referent": ("""
  var r: Ref := new(value)
  r.value := Num(7)
  fold physical(r, IntType())
  fold group(Num(7), IntType())
  fold group(Ptr(r), RefType(IntType()))
  fold group(Ptr(r), RefType(IntType()))
""", "There might be insufficient permission to access physical(get_Snap_address(s), get_Type_inner(t))"),
        "missing_pair_field": ("""
  var r: Ref := new(value)
  r.value := Num(7)
  fold physical(r, IntType())
  fold group(Num(7), IntType())
  fold group(Ptr(r), RefType(IntType()))
  fold group(Pair(Ptr(r), Ptr(r)), PairType(RefType(IntType()), RefType(IntType())))
""", "There might be insufficient permission to access group(get_Snap_second(s), get_Type_right(t))"),
        "state_without_permission": ("""
  var s: Snap
  var t: Type
  var observed: State := state(s, t)
""", "There might be insufficient permission to access group(s, t)"),
        "identity_without_permission": ("""
  var s: Snap
  var t: Type
  s := generic_identity(s, t)
""", "There might be insufficient permission to access group(s, t)"),
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
    with tempfile.TemporaryDirectory(prefix="stdlib-im4-groups-", dir=root / "target") as temp:
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
    print(f"All generic-resource checks passed ({len(cases)} checks).")


if __name__ == "__main__":
    main()
