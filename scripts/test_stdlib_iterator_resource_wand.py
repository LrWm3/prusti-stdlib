"""Check joint storage/referent wand composition and its negative controls.

This standalone constructor model has checked bodies and adds no axioms.
It isolates proof composition from Rust encoding; passing it does not establish
whole-loop referent values or the IM5 acceptance gate.
"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--case", choices=("valid", "wrong_state", "false"))
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    model = root / "prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/joint_resource_wand.vpr"
    program = model.read_text()
    state = "state(source_contents(owner),j) == old[lhs](state(tail_contents(owner),j)) + 0"
    consistency = "// CONSISTENCY_CHECK"
    if program.count(state) != 1 or program.count(consistency) != 1:
        raise ValueError("joint resource model markers have changed")
    java_home = os.environ.get("JAVA_HOME")
    java = str(Path(java_home) / "bin/java") if java_home else "java"
    env = os.environ.copy()
    env["Z3_EXE"] = str(root / "viper_tools/z3/bin/z3")
    command = [
        java, "-Xss512m", "-cp", str(root / "viper_tools/backends/viperserver.jar"),
        "viper.silicon.SiliconRunner", "--logLevel", "ERROR",
        "--enableMoreCompleteExhale", "--disableTerminationPlugin",
        "--assertTimeout", "30000", "--proverEnableTimeBounds",
        "--numberOfParallelVerifiers", "1", "--timeout", "180",
    ]
    logs = root / "target/stdlib-im5-joint-resource-wand-logs"
    logs.mkdir(parents=True, exist_ok=True)
    results = []
    for case in ([args.case] if args.case else ["valid", "wrong_state", "false"]):
        modified = program
        if case == "wrong_state":
            modified = modified.replace(state, state[:-1] + "1")
        elif case == "false":
            modified = modified.replace(consistency, "assert false")
        path = logs / f"{case}.vpr"
        path.write_text(modified)
        print(f"Running {case} ...", flush=True)
        start = time.monotonic()
        try:
            result = subprocess.run(command + [str(path)], cwd=root, env=env, text=True,
                                    stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=240)
            output = result.stdout
            errors = re.findall(r"^\s*\[\d+\] (.+)$", output, re.MULTILINE)
            if case == "valid":
                passed = result.returncode == 0 and "finished verification successfully" in output and not errors
            elif case == "false":
                line = modified[:modified.index("assert false")].count("\n") + 1
                passed = (result.returncode == 1 and len(errors) == 1
                          and errors[0].startswith("Assert might fail.")
                          and f"{case}.vpr@{line}." in errors[0])
            else:
                line = modified[:modified.index("  package (acc(Tail(owner))")].count("\n") + 1
                passed = (result.returncode == 1 and len(errors) == 1
                          and errors[0].startswith("Packaging wand might fail.")
                          and f"{case}.vpr@{line}." in errors[0])
            passed = passed and not re.search(r"Verification timed out|internal error|Exception|Parsing error", output)
        except subprocess.TimeoutExpired as error:
            output = error.stdout or b""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            output += "\nTest command exceeded 240 seconds.\n"
            passed = False
        elapsed = time.monotonic() - start
        (logs / f"{case}.log").write_text(output)
        results.append({"case": case, "passed": passed, "seconds": round(elapsed, 2)})
        print(f"{case}: {'PASS' if passed else 'FAIL'} ({elapsed:.2f}s)", flush=True)
    (logs / "results.json").write_text(json.dumps(results, indent=2) + "\n")
    if not all(result["passed"] for result in results):
        raise SystemExit(1)
    print("Selected joint resource wand checks passed.")


if __name__ == "__main__":
    main()
