"""Run the cross-crate stdlib contract milestones, including diagnostic checks.

From the repository root, after building Prusti:
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite sequence
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite vec
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite collections
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite borrow
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator_resources
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator_generic_resources
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite hash
    ./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite resources

For a focused client regression, add --case 'name_*' --skip-library.
CARGO_TARGET_DIR selects the directory containing the built Prusti executable;
each run still uses a fresh temporary Cargo target for the fixtures.
"""

import argparse
import ctypes
import fnmatch
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import subprocess
import tempfile
import time


def process_start(pid):
    try:
        return Path(f"/proc/{pid}/stat").read_text().rsplit(")", 1)[1].split()[19]
    except (FileNotFoundError, ProcessLookupError, IndexError):
        return None


def discover_children(owned):
    for pid, start in list(owned.items()):
        if start is None or process_start(pid) != start:
            continue
        for task in Path(f"/proc/{pid}/task").glob("*"):
            try:
                children = (task / "children").read_text().split()
            except (FileNotFoundError, ProcessLookupError):
                continue
            for child in children:
                child = int(child)
                start = process_start(child)
                if start is not None:
                    owned.setdefault(child, start)


def run_owned(command, cwd, env, timeout):
    """Reap our child and kill observed descendants identified by PID and start time."""
    if not Path("/proc/self/task").exists():
        # Portable fallback retains subprocess.run timeout semantics; only the
        # directly owned process is guaranteed stopped on non-Linux platforms.
        try:
            result = subprocess.run(command, cwd=cwd, env=env, stdout=subprocess.PIPE,
                                    stderr=subprocess.STDOUT, text=True, timeout=timeout)
            return result.returncode, False, result.stdout
        except subprocess.TimeoutExpired as error:
            output = error.stdout or b""
            if isinstance(output, bytes):
                output = output.decode(errors="replace")
            return None, True, output + f"\nTest command exceeded {timeout} seconds.\n"
    # Adopt orphaned descendants so this harness can reap them after timeout.
    if ctypes.CDLL(None, use_errno=True).prctl(36, 1, 0, 0, 0) != 0:  # PR_SET_CHILD_SUBREAPER
        raise OSError(ctypes.get_errno(), "cannot enable child subreaper")
    with tempfile.TemporaryFile(mode="w+") as output:
        process = subprocess.Popen(command, cwd=cwd, env=env, stdout=output,
                                   stderr=subprocess.STDOUT, text=True, start_new_session=True)
        owned = {process.pid: process_start(process.pid)}
        deadline = time.monotonic() + timeout
        timed_out = False
        while process.poll() is None:
            discover_children(owned)
            if time.monotonic() >= deadline:
                timed_out = True
                break
            time.sleep(0.05)
        if timed_out:
            discover_children(owned)
            for pid, start in reversed(list(owned.items())):
                if start is not None and process_start(pid) == start:
                    try:
                        os.kill(pid, signal.SIGKILL)
                    except ProcessLookupError:
                        pass
        process.wait()
        if timed_out:
            deadline = time.monotonic() + 2
            pending = {pid: start for pid, start in owned.items() if pid != process.pid}
            while pending and time.monotonic() < deadline:
                for pid, start in list(pending.items()):
                    if process_start(pid) != start:
                        del pending[pid]
                        continue
                    try:
                        reaped, _ = os.waitpid(pid, os.WNOHANG)
                    except ChildProcessError:
                        continue  # Its still-living parent must reap it first.
                    if reaped:
                        del pending[pid]
                if pending:
                    time.sleep(0.01)
        output.seek(0)
        text = output.read()
        if timed_out:
            text += f"\nTest command exceeded {timeout} seconds.\n"
        return process.returncode, timed_out, text


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def git_revision(root, reference="HEAD"):
    result = subprocess.run(["git", "rev-parse", "--verify", reference], cwd=root, text=True,
                            stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    return result.stdout.strip() if result.returncode == 0 else None


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--log-dir", type=Path, help="dedicated output directory for this run or shard")
    parser.add_argument("--release", action="store_true", help="use release Prusti binaries")
    parser.add_argument("--suite", choices=("extern", "sequence", "vec", "collections", "borrow", "iterator", "iterator_resources", "iterator_generic_resources", "hash", "resources"), default="extern")
    parser.add_argument("--backend", choices=("Silicon", "Ferrum"), default="Silicon", help="verification backend; requires a driver built with that backend")
    parser.add_argument("--case", action="append", help="select binary names by glob; may be repeated")
    parser.add_argument("--skip-library", action="store_true", help="verify selected clients without separately verifying the spec library")
    parser.add_argument("--timeout", type=int, default=180, help="verification time limit per generated program in seconds (default: 180)")
    parser.add_argument("--solver-timeout-ms", type=int, help="time/resource budget per solver assertion in milliseconds")
    parser.add_argument("--solver-check-timeout-ms", type=int, help="budget per solver check, including wand matching, in milliseconds")
    parser.add_argument("--solver-time-bounds", action="store_true", help="use elapsed-time bounds instead of Z3 resource bounds")
    args = parser.parse_args()
    if args.timeout <= 0:
        parser.error("--timeout must be positive")
    if args.solver_timeout_ms is not None and args.solver_timeout_ms <= 0:
        parser.error("--solver-timeout-ms must be positive")
    if args.solver_check_timeout_ms is not None and args.solver_check_timeout_ms <= 0:
        parser.error("--solver-check-timeout-ms must be positive")
    command_timeout = max(300, args.timeout + 120)
    root = Path(__file__).resolve().parent.parent
    sequence = args.suite == "sequence"
    fixture = root / "prusti-tests/tests/cargo_verify" / ("stdlib_" + args.suite + "_spec")
    profile = "release" if args.release else "debug"
    executable = "cargo-prusti.exe" if os.name == "nt" else "cargo-prusti"
    prusti = Path(os.environ.get("CARGO_TARGET_DIR", root / "target")).resolve() / profile / executable
    if not prusti.is_file():
        parser.error("build Prusti first with ./x.py build --all (and --release if selected)")
    sources = sorted((fixture / "src/bin").glob("*.rs"))
    if args.case:
        sources = [source for source in sources if any(fnmatch.fnmatchcase(source.stem, pattern) for pattern in args.case)]
        if not sources:
            parser.error("no binaries match the selected --case patterns")

    toolchain = re.search(r'channel\s*=\s*"([^"]+)"', (root / "rust-toolchain").read_text())[1]
    log_dir = args.log_dir.resolve() if args.log_dir else root / "target" / ("stdlib-" + args.suite + "-spec-logs") / args.backend.lower()
    log_dir.mkdir(parents=True, exist_ok=True)
    results = []
    artifacts = {}
    for artifact in (prusti, prusti.with_name("prusti-driver" + prusti.suffix),
                     prusti.with_name("prusti-server" + prusti.suffix)):
        if artifact.is_file():
            artifacts[artifact.name] = {"path": str(artifact), "sha256": sha256(artifact)}
    for name in ("Z3_EXE", "JAVA_HOME", "VIPER_HOME"):
        value = os.environ.get(name)
        if value:
            path = Path(value)
            artifacts[name] = {"path": value, "sha256": sha256(path) if path.is_file() else None}
            if name == "JAVA_HOME":
                java = path / "bin" / ("java.exe" if os.name == "nt" else "java")
                if java.is_file():
                    artifacts["java"] = {"path": str(java), "sha256": sha256(java)}
    metadata = {"artifacts": artifacts, "toolchain": toolchain, "backend": args.backend, "suite": args.suite,
                "source_revision": git_revision(root), "merge_revision": git_revision(root, "MERGE_HEAD"), "driver": str(prusti),
                "driver_sha256": sha256(prusti), "harness_sha256": sha256(Path(__file__)),
                "case_patterns": args.case, "spec_library_selected": not args.skip_library,
                "verifier_timeout": args.timeout, "solver_timeout_ms": args.solver_timeout_ms, "solver_check_timeout_ms": args.solver_check_timeout_ms,
                "solver_time_bounds": args.solver_time_bounds,
                "parallel_verifiers": 1, "parallelize_branches": False}

    def save_results():
        temporary = log_dir / "results.json.tmp"
        temporary.write_text(json.dumps({**metadata, "cases": results}, indent=2) + "\n")
        temporary.replace(log_dir / "results.json")

    save_results()

    def run(name, command, env, expected=None, location=None, notes=()):
        print("Running {} ...".format(name), flush=True)
        start = time.monotonic()
        returncode, timed_out, output = run_owned(command, fixture, env, command_timeout)
        errors = re.findall(r"^error(?:\[[^\]]+\])?: (.+)$", output, re.MULTILINE)
        errors = [e for e in errors if not e.startswith(("could not compile", "aborting due to"))]
        if expected is None:
            passed = returncode == 0 and not errors
        else:
            passed = (returncode == 101 and errors == [expected]
                      and re.search(location, output) is not None
                      and all(re.search(r"^\s*(?:= )?note: .*" + re.escape(note), output, re.MULTILINE) for note in notes))
        passed = passed and not timed_out and not re.search(
            r"\[Prusti: internal error\]|internal compiler error|"
            r"thread .* panicked|Verification timed out", output)
        if expected is None or not expected.startswith("[Prusti: unsupported feature]"):
            passed = passed and "[Prusti: unsupported feature]" not in output
        elapsed = round(time.monotonic() - start, 2)
        log_path = log_dir / (name + ".log")
        log_path.write_text(output)
        source = fixture / "src/bin" / (name.removeprefix("runtime-") + ".rs")
        results.append({"case": name, "passed": passed, "seconds": elapsed,
                        "command": command, "exit_code": returncode, "timed_out": timed_out,
                        "expected_diagnostic": expected, "expected_location": location,
                        "expected_notes": list(notes),
                        "source_sha256": sha256(source) if source.is_file() else None})
        save_results()
        print("{}: {} ({:.2f}s); {}".format(name, "PASS" if passed else "FAIL", elapsed, log_path), flush=True)
        if not passed:
            print(output, flush=True)
        return passed

    # A fresh target directory forces real verification on every invocation.
    # Only artifacts created by this run are removed; logs remain for diagnosis.
    (root / "target").mkdir(exist_ok=True)
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
            "PRUSTI_VIPER_BACKEND": args.backend,
            "PRUSTI_CHECK_OVERFLOWS": "true",
            "PRUSTI_CHECK_PANICS": "true",
            "PRUSTI_NO_VERIFY_DEPS": "true",
            "PRUSTI_ENABLE_CACHE": "false",
            # The timeout covers the whole generated program, including
            # generic conversion methods and the untrusted iterator body.
            "PRUSTI_EXTRA_VERIFIER_ARGS": "--timeout={} --numberOfParallelVerifiers 1".format(args.timeout),
            "PRUSTI_LOG_DIR": str(log_dir / "verifier"),
        })
        if args.backend == "Ferrum":
            env["PRUSTI_EXTRA_VERIFIER_ARGS"] += " --parallelizeBranches=false"
        if args.solver_timeout_ms is not None:
            env["PRUSTI_ASSERT_TIMEOUT"] = str(args.solver_timeout_ms)
        if args.solver_check_timeout_ms is not None:
            env["PRUSTI_EXTRA_VERIFIER_ARGS"] += " --checkTimeout {}".format(args.solver_check_timeout_ms)
        if args.solver_time_bounds:
            env["PRUSTI_EXTRA_VERIFIER_ARGS"] += " --proverEnableTimeBounds"
        compiled = run("rust-check", ["cargo", "check", "--bins"], env)
        if compiled:
            if args.suite == "extern":
                run("rust-core-only", ["cargo", "check", "-p", "stdlib_core_only"], env)
                run("core-only", [str(prusti), "-p", "stdlib_core_only"], env)
            library = {"extern": "stdlib_specs", "sequence": "sequence_specs", "vec": "vec_specs", "collections": "collection_specs", "borrow": "borrow_specs", "iterator": "iterator_specs", "iterator_resources": "iterator_resource_specs", "iterator_generic_resources": "iterator_generic_resource_specs", "hash": "key_specs", "resources": "resource_specs"}[args.suite]
            if not args.skip_library:
                run("spec-library", [str(prusti), "-p", library], env)
            positive = [p for p in sources if p.stem == "pass" or p.stem.endswith("_pass")]
            for source in positive:
                run("runtime-" + source.stem, ["cargo", "run", "--bin", source.stem], env)
            for source in positive + sorted(set(sources) - set(positive)):
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

            if sequence and any(source.stem == "triggers_pass" for source in positive):
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
                    save_results()
                    print("Viper {} trigger: {} ({})".format(quantifier, "PASS" if passed else "FAIL", program), flush=True)

    save_results()
    if not all(result["passed"] for result in results):
        raise SystemExit(1)
    scope = "Selected" if args.case or args.skip_library else "All"
    print("{} stdlib {} cases passed ({} checks).".format(scope, args.suite, len(results)))


if __name__ == "__main__":
    main()
