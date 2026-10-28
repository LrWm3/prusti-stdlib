"""Check the prefix-restoration model and deliberately invalid clients.

From the repository root, with the pinned Viper tools and Java installed:
    ./x.py exec python3 scripts/test_stdlib_prefix_wand.py
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
    source = root / "prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/prefix_wand.vpr"
    logs = root / "target/stdlib-im1-prefix-logs"
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
    # Each negative runs against the verified contracts of the model. Both
    # array states are reachable with a single concrete, owned location.
    negatives = {
        "wrong_prefix": ("fill_prefix", 1, 1, "values(xs)[0] == 8", "Assertion values(xs)[0] == 8 might not hold"),
        "wrong_suffix": ("fill_prefix", 0, 1, "values(xs)[0] == 7", "Assertion values(xs)[0] == 7 might not hold"),
        "wrong_increment": ("increment_prefix", 1, 1, "values(xs)[0] == 3", "Assertion values(xs)[0] == 3 might not hold"),
        "overflow": ("increment_prefix", 1, 4294967295, None, "The precondition of method increment_prefix might not hold"),
        "false": ("fill_prefix", 0, 1, "false", "Assertion false might not hold"),
    }
    cases = [("model", None, None)] + [
        (name, (method, stop, initial, assertion), expected)
        for name, (method, stop, initial, assertion, expected) in negatives.items()
    ]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", choices=[name for name, _, _ in cases])
    args = parser.parse_args()
    if args.case:
        cases = [case for case in cases if case[0] == args.case]
    passed = True
    with tempfile.TemporaryDirectory(prefix="stdlib-im1-prefix-", dir=root / "target") as temp:
        for name, negative, expected in cases:
            program = source.read_text()
            if negative is not None:
                method, stop, initial, assertion = negative
                check = f"  assert {assertion}\n" if assertion is not None else ""
                program += f"""
method negative_witness() {{
  var item: Ref := new(value)
  item.value := {initial}
  var xs: Seq[Ref] := Seq(item)
  fold elements(xs)
  {method}(xs, {stop})
{check}
}}
"""
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
    print(f"All prefix-wand checks passed ({len(cases)} checks).")


if __name__ == "__main__":
    main()
