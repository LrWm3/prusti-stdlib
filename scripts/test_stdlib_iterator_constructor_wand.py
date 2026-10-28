"""Check the constructor's value relation in an emitted iterator proof.

Pass the program-check.vpr dumped while verifying mut_native_tail_suffix_pass.
The probe uses the emitted predicates and constructor pledge, without adding
axioms. It checks the first loop-wand package's proof script, not preservation
of the value relation through the loop.
"""

import argparse
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


def instrument(program: str, operation: str) -> tuple[str, int]:
    # These locals and labels identify the pinned Rust fixture's constructor
    # package. Refuse to run if its generated shape has changed.
    label = "outer_package2"
    marker = f"  label {label}\n"
    method = "method m_visit(_0p: Ref, _1p: Ref)"
    if program.count(marker) != 1 or method not in program:
        raise ValueError("the emitted program does not match the iterator fixture")
    method_start = program.index(method)
    next_method = program.find("\nmethod ", method_start + len(method))
    package_start = program.index(marker)
    if package_start < method_start or (next_method != -1 and package_start >= next_method):
        raise ValueError("the constructor package is not inside m_visit")
    start = program.index("{", package_start)
    depth = 0
    end = None
    for index in range(start, len(program)):
        if program[index] == "{":
            depth += 1
        elif program[index] == "}":
            depth -= 1
            if depth == 0:
                end = index
                break
    if end is None:
        raise ValueError("constructor package block is incomplete")
    block = program[start:end]
    if "iter_mut at bb6" not in block or "make_concrete_Slice" not in block:
        raise ValueError("the expected constructor wand is absent")

    owner = "old[outer_package2](p_Ref_mutable_snap(_1p, s_Slice_type(s_UInt_u32_type())))"
    tail = ("old[outer_package2](p_std$col$$col$slice$col$$col$IterMut_snap(_14p, "
            "s_UInt_u32_type())).s_std$col$$col$slice$col$$col$IterMut_3")
    lhs = (f"s_Slice_to_seq(p_Slice_snap({owner}.s_Ref_mutable_0, s_UInt_u32_type()), "
           f"s_UInt_usize_value({owner}.s_Ref_mutable_1.make_concrete_s_UInt_usize))")
    rhs = (f"old[lhs](s_Slice_to_seq(p_Param_snap({tail}.s_Ref_mutable_0, "
           "s_Slice_type(s_UInt_u32_type())).make_concrete_s_Slice, "
           f"s_UInt_usize_value({tail}.s_Ref_mutable_1.make_concrete_s_UInt_usize)))")
    assertion = f"  assert {lhs} {operation} {rhs}\n  "
    return program[:end] + assertion + program[end:], program[:end].count("\n") + 1


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("viper_program", type=Path)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    program = args.viper_program.read_text()
    java_home = os.environ.get("JAVA_HOME")
    java = str(Path(java_home) / "bin/java") if java_home else "java"
    env = os.environ.copy()
    env["Z3_EXE"] = str(root / "viper_tools/z3/bin/z3")
    command = [
        java, "-Xss512m", "-cp", str(root / "viper_tools/backends/viperserver.jar"),
        "viper.silicon.SiliconRunner", "--logLevel", "ERROR", "--enableMoreCompleteExhale",
        "--disableTerminationPlugin", "--assertTimeout", "10000", "--proverConfigArgs",
        "smt.qi.eager_threshold=1000 model.partial=false smt.arith.solver=6",
        "--numberOfParallelVerifiers", "1", "--timeout", "180", "--includeMethods", "m_visit",
    ]
    logs = root / "target/stdlib-im2-constructor-wand-logs"
    logs.mkdir(parents=True, exist_ok=True)
    all_ok = True
    with tempfile.TemporaryDirectory(prefix="stdlib-im2-constructor-", dir=root / "target") as tmp:
        for name, operation in (("valid", "=="), ("wrong", "!=")):
            modified, line = instrument(program, operation)
            path = Path(tmp) / f"{name}.vpr"
            path.write_text(modified)
            print(f"Running {name} ...", flush=True)
            start = time.monotonic()
            try:
                result = subprocess.run(command + [str(path)], cwd=root, env=env, text=True,
                                        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=240)
                output = result.stdout
                errors = re.findall(r"^\s*\[\d+\] (.+)$", output, re.MULTILINE)
                if name == "valid":
                    ok = result.returncode == 0 and "finished verification successfully" in output and not errors
                else:
                    ok = (result.returncode == 1 and len(errors) == 1
                          and errors[0].startswith("Assert might fail.")
                          and f"{name}.vpr@{line}." in errors[0])
                ok = ok and not re.search(r"Verification timed out|internal error|Exception|Parsing error", output)
            except subprocess.TimeoutExpired as error:
                output = error.stdout or b""
                if isinstance(output, bytes):
                    output = output.decode(errors="replace")
                output += "\nTest command exceeded 240 seconds.\n"
                ok = False
            (logs / f"{name}.log").write_text(output)
            print(f"{name}: {'PASS' if ok else 'FAIL'} ({time.monotonic() - start:.2f}s)", flush=True)
            all_ok &= ok
    if not all_ok:
        raise SystemExit(1)
    print("Both constructor-wand checks passed.")


if __name__ == "__main__":
    main()
