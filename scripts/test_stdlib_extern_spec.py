"""Run the cross-crate stdlib contract milestones, including diagnostic checks.

From the repository root, after building Prusti:
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite sequence
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite vec
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite collections
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite borrow
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator
"""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true", help="use release Prusti binaries")
    parser.add_argument("--suite", choices=("extern", "sequence", "vec", "collections", "borrow", "iterator"), default="extern")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    sequence = args.suite == "sequence"
    fixture = root / "prusti-tests/tests/cargo_verify" / ("stdlib_" + args.suite + "_spec")
    profile = "release" if args.release else "debug"
    executable = "cargo-prusti.exe" if os.name == "nt" else "cargo-prusti"
    prusti = root / "target" / profile / executable
    if not prusti.is_file():
        parser.error("build Prusti first with ./x.py build --all (and --release if selected)")

    toolchain = re.search(r'channel\s*=\s*"([^"]+)"', (root / "rust-toolchain").read_text())[1]
    log_dir = root / "target" / ("stdlib-" + args.suite + "-spec-logs")
    log_dir.mkdir(parents=True, exist_ok=True)
    results = []

    def run(name, command, env, expected=None, location=None, notes=()):
        print("Running {} ...".format(name), flush=True)
        start = time.monotonic()
        try:
            result = subprocess.run(
                command, cwd=fixture, env=env, text=True,
                stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=300,
            )
            output = result.stdout
            errors = re.findall(r"^error(?:\[[^\]]+\])?: (.+)$", output, re.MULTILINE)
            # Cargo's final compilation summary is not a primary diagnostic.
            errors = [e for e in errors if not e.startswith(("could not compile", "aborting due to"))]
            if expected is None:
                passed = result.returncode == 0 and not errors
            else:
                passed = (
                    result.returncode == 101
                    and errors == [expected]
                    and re.search(location, output) is not None
                    and all(re.search(r"^\s*(?:= )?note: .*" + re.escape(note), output, re.MULTILINE) for note in notes)
                )
            passed = passed and not re.search(
                r"\[Prusti: internal error\]|internal compiler error|"
                r"thread .* panicked|Verification timed out", output,
            )
            if expected is None or not expected.startswith("[Prusti: unsupported feature]"):
                passed = passed and "[Prusti: unsupported feature]" not in output
        except subprocess.TimeoutExpired as error:
            output = error.stdout or b""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            output += "\nTest command exceeded 300 seconds.\n"
            passed = False
        elapsed = round(time.monotonic() - start, 2)
        log_path = log_dir / (name + ".log")
        log_path.write_text(output)
        results.append({"case": name, "passed": passed, "seconds": elapsed})
        print("{}: {} ({:.2f}s); {}".format(name, "PASS" if passed else "FAIL", elapsed, log_path), flush=True)
        if not passed:
            print(output, flush=True)
        return passed

    # A fresh target directory forces real verification on every invocation.
    # Only artifacts created by this run are removed; logs remain for diagnosis.
    with tempfile.TemporaryDirectory(prefix="stdlib-extern-spec-", dir=root / "target") as target:
        env = os.environ.copy()
        for key in list(env):
            if key.startswith(("PRUSTI_", "DEFAULT_PRUSTI_")) or key in (
                "RUSTC", "RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER", "RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS",
            ):
                del env[key]
        env.update({
            "RUSTUP_TOOLCHAIN": toolchain,
            "CARGO_TARGET_DIR": target,
            "CARGO_TERM_COLOR": "never",
            "PRUSTI_VERIFY_MODE": "verify",
            "PRUSTI_CHECK_OVERFLOWS": "true",
            "PRUSTI_CHECK_PANICS": "true",
            "PRUSTI_NO_VERIFY_DEPS": "true",
            "PRUSTI_ENABLE_CACHE": "false",
            "PRUSTI_EXTRA_VERIFIER_ARGS": "--timeout=90",
            "PRUSTI_LOG_DIR": str(log_dir / "verifier"),
        })
        compiled = run("rust-check", ["cargo", "check", "--bins"], env)
        if compiled:
            if args.suite == "extern":
                run("rust-core-only", ["cargo", "check", "-p", "stdlib_core_only"], env)
                run("core-only", [str(prusti), "-p", "stdlib_core_only"], env)
            library = {"extern": "stdlib_specs", "sequence": "sequence_specs", "vec": "vec_specs", "collections": "collection_specs", "borrow": "borrow_specs", "iterator": "iterator_specs"}[args.suite]
            run("spec-library", [str(prusti), "-p", library], env)
            sources = fixture / "src/bin"
            positive = sorted(p for p in sources.glob("*.rs") if p.stem == "pass" or p.stem.endswith("_pass"))
            for source in positive:
                run("runtime-" + source.stem, ["cargo", "run", "--bin", source.stem], env)
            for source in positive + sorted(set(sources.glob("*.rs")) - set(positive)):
                annotations = [
                    (i, match[1], match[2])
                    for i, line in enumerate(source.read_text().splitlines(), 1)
                    if (match := re.search(r"// expected-(error|incorrect|parse|unsupported): (.+)", line))
                ]
                notes = re.findall(r"// expected-note: (.+)", source.read_text())
                if source in positive:
                    assert not annotations
                    expected, location = None, None
                else:
                    assert len(annotations) == 1, "each negative case needs one expected-error annotation"
                    line, kind, message = annotations[0]
                    category = {
                        "error": "verification error", "incorrect": "invalid specification",
                        "unsupported": "unsupported feature", "parse": None,
                    }[kind]
                    expected = message if kind == "parse" else "[Prusti: {}] {}".format(category, message)
                    location = r"--> [^\n]*[/\\]" + re.escape(source.name) + ":" + str(line) + ":"
                case_env = env.copy()
                if args.suite != "extern" and source in positive:
                    case_env["PRUSTI_DUMP_VIPER_PROGRAM"] = "true"
                    case_env["PRUSTI_LOG_DIR"] = str(log_dir / "verifier" / source.stem)
                    program = Path(case_env["PRUSTI_LOG_DIR"]) / "viper_program/program-check.vpr"
                    program.unlink(missing_ok=True)
                run(source.stem, [str(prusti), "--bin", source.stem], case_env, expected, location, notes)

            if sequence:
                program = log_dir / "verifier/triggers_pass/viper_program/program-check.vpr"
                viper = program.read_text() if program.is_file() else ""
                for quantifier in ("forall", "exists"):
                    # The marker occurs only in explicit trigger annotations,
                    # never in a quantified body; automatic inference cannot
                    # account for it. Check the trigger braces, not definitions.
                    passed = re.search(
                        r"\b" + quantifier + r"\b[^{}]*\{[^{}]*explicit_marker[^{}]*\}", viper,
                    ) is not None
                    results.append({"case": "viper-trigger-" + quantifier, "passed": passed, "seconds": 0})
                    print("Viper {} trigger: {} ({})".format(quantifier, "PASS" if passed else "FAIL", program), flush=True)

    (log_dir / "results.json").write_text(json.dumps({"toolchain": toolchain, "cases": results}, indent=2) + "\n")
    if not all(result["passed"] for result in results):
        raise SystemExit(1)
    print("All stdlib {} cases passed ({} checks).".format(args.suite, len(results)))


if __name__ == "__main__":
    main()
