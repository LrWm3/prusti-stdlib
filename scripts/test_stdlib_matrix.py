"""Run the complete stdlib matrix serially, preserving coverage and failed runs.

Use --source-root with an immutable checkout and --driver-dir with binaries
built from that revision. --resume reuses only complete, matching results.
Monitoring starts frequently and backs off to at most an hour while healthy;
child completion wakes the dispatcher immediately.
"""

import argparse
from collections import Counter
from datetime import datetime, timezone
import fnmatch
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time


SUITES = (
    "extern", "sequence", "vec", "collections", "resources", "hash",
    "borrow", "iterator", "iterator_resources", "iterator_generic_resources",
    "iterator_prefix_resources",
)
HEAVY_SUITES = frozenset((
    "sequence", "resources", "iterator_resources", "iterator_generic_resources",
    "iterator_prefix_resources",
))
HEAVY_ITERATORS = (
    "mut_native_resource_prefix_*", "mut_native_resource_generic_prefix_*",
    "mut_native_resource_pair_prefix_*", "mut_native_resource_early_*",
)


def now():
    return datetime.now(timezone.utc).isoformat()


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, indent=2) + "\n")
    temporary.replace(path)


def revision(root):
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=root, text=True,
    ).strip()


def plan(root, suites, backends):
    jobs = []
    for suite in suites:
        folder = root / "prusti-tests/tests/cargo_verify" / f"stdlib_{suite}_spec"
        cases = sorted(p.stem for p in (folder / "src/bin").glob("*.rs"))
        if not cases:
            raise ValueError(f"No client cases found for {suite}")
        if suite == "iterator":
            heavy = [name for name in cases if any(
                fnmatch.fnmatchcase(name, pattern) for pattern in HEAVY_ITERATORS
            )]
            groups = [("ordinary", [name for name in cases if name not in heavy], 180, True),
                      ("heavy", heavy, 600, False)]
        else:
            groups = [(None, cases, 600 if suite in HEAVY_SUITES else 180, True)]
        for group, selected, timeout, library in groups:
            if not selected:
                continue
            for backend in backends:
                key = "-".join(filter(None, (suite, group, backend.lower())))
                jobs.append({"key": key, "suite": suite, "backend": backend,
                             "cases": selected, "timeout": timeout,
                             "library": library, "filtered": group is not None})
    return jobs


def expected_checks(job):
    names = ["rust-check", *job["cases"]]
    names += ["runtime-" + name for name in job["cases"]
              if name == "pass" or name.endswith("_pass")]
    if job["library"]:
        names.append("spec-library")
    if job["suite"] == "extern":
        names += ["rust-core-only", "core-only"]
    if job["suite"] == "sequence" and "triggers_pass" in job["cases"]:
        names += ["viper-trigger-forall", "viper-trigger-exists"]
    return Counter(names)


def assess(data, job, source_hashes, driver_hash):
    cases = data.get("cases", [])
    actual = Counter(case["case"] for case in cases)
    expected = expected_checks(job)
    metadata = (
        data.get("suite") == job["suite"] and data.get("backend") == job["backend"]
        and data.get("verifier_timeout") == job["timeout"]
        and data.get("solver_timeout_ms") == 30000
        and data.get("solver_check_timeout_ms") == 100
        and data.get("solver_time_bounds") is True
        and data.get("parallel_verifiers") == 1
        and data.get("parallelize_branches") is False
        and data.get("spec_library_selected") == job["library"]
        and data.get("artifacts", {}).get("prusti-driver", {}).get("sha256") == driver_hash
    )
    prefix = f"prusti-tests/tests/cargo_verify/stdlib_{job['suite']}_spec/src/bin/"
    hashes_match = all(
        case.get("source_sha256") == source_hashes.get(prefix + case["case"] + ".rs")
        and prefix + case["case"] + ".rs" in source_hashes
        for case in cases if case["case"] in job["cases"]
    )
    return {"complete": actual == expected and metadata and hashes_match,
            "metadata_match": metadata, "source_hashes_match": hashes_match,
            "missing": list((expected - actual).elements()),
            "unexpected": list((actual - expected).elements()),
            "checks": len(cases), "passed": sum(case["passed"] for case in cases),
            "failures": [case["case"] for case in cases if not case["passed"]]}


def source_manifest(root, suites):
    tracked = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"], cwd=root,
    ).decode().split("\0")
    prefixes = ["prusti-contracts/", "rust-toolchain", "viper-toolchain",
                "scripts/test_stdlib_extern_spec.py"]
    prefixes += [f"prusti-tests/tests/cargo_verify/stdlib_{suite}_spec/" for suite in suites]
    return {name: digest(root / name) for name in tracked
            if name and any(name.startswith(prefix) for prefix in prefixes)
            and (root / name).is_file()}


def read_results(path):
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        return {"cases": []}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--driver-dir", type=Path)
    parser.add_argument("--log-dir", type=Path, required=True)
    parser.add_argument("--suite", action="append", choices=SUITES)
    parser.add_argument("--backend", action="append", choices=("Ferrum", "Silicon"))
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--resume", action="store_true")
    parser.add_argument("--list", action="store_true", help="print the coverage plan without running it")
    args = parser.parse_args()
    root = args.source_root.resolve()
    suites = list(dict.fromkeys(args.suite or SUITES))
    backends = list(dict.fromkeys(args.backend or ("Ferrum", "Silicon")))
    jobs = plan(root, suites, backends)
    if args.list:
        print(json.dumps(jobs, indent=2))
        return
    if args.driver_dir is None:
        parser.error("--driver-dir must identify the build being qualified")
    driver_dir = args.driver_dir.resolve()
    profile = "release" if args.release else "debug"
    artifacts = {name: digest(driver_dir / profile / name)
                 for name in ("cargo-prusti", "prusti-driver", "prusti-server")}
    log_dir = args.log_dir.resolve()
    log_dir.mkdir(parents=True, exist_ok=True)
    manifest = {"source_root": str(root), "source_revision": revision(root),
                "sources": source_manifest(root, suites), "artifacts": artifacts,
                "driver_dir": str(driver_dir), "jobs": jobs,
                "runner_sha256": digest(Path(__file__))}
    manifest_path = log_dir / "manifest.json"
    if manifest_path.exists():
        if not args.resume or json.loads(manifest_path.read_text()) != manifest:
            parser.error("existing matrix manifest differs, or --resume was not supplied; use a new log directory")
    else:
        write_json(manifest_path, manifest)
    status = {"started_at": now(), "state": "running", "completed": [], "active": None}

    def save():
        status["updated_at"] = now()
        write_json(log_dir / "status.json", status)

    def check_sources():
        changed = [name for name, value in manifest["sources"].items()
                   if not (root / name).is_file() or digest(root / name) != value]
        if changed:
            raise RuntimeError("matrix source snapshot changed: " + ", ".join(changed))

    env = os.environ.copy()
    env["CARGO_TARGET_DIR"] = str(driver_dir)
    harness = root / "scripts/test_stdlib_extern_spec.py"
    for job in jobs:
        check_sources()
        job_dir = log_dir / job["key"]
        result_path = job_dir / "results.json"
        if result_path.exists():
            if not args.resume:
                parser.error(f"results already exist for {job['key']}; use --resume or a new log directory")
            report = assess(read_results(result_path), job, manifest["sources"], artifacts["prusti-driver"])
            if report["complete"]:
                status["completed"].append({"job": job["key"], "reused": True, **report})
                save()
                print(f"Reused complete {job['key']}: {report['passed']}/{report['checks']} checks", flush=True)
                continue
            # Preserve an incomplete attempt before trying it again.
            archived = log_dir / (job["key"] + "-incomplete-" + str(time.time_ns()))
            job_dir.rename(archived)
        job_dir.mkdir(parents=True, exist_ok=True)
        command = [sys.executable, str(harness), "--suite", job["suite"],
                   "--backend", job["backend"], "--timeout", str(job["timeout"]),
                   "--solver-timeout-ms", "30000", "--solver-check-timeout-ms", "100",
                   "--solver-time-bounds", "--log-dir", str(job_dir)]
        if args.release:
            command.append("--release")
        if not job["library"]:
            command.append("--skip-library")
        if job["filtered"]:
            for name in job["cases"]:
                command += ["--case", name]
        status["active"] = {"job": job["key"], "started_at": now(), "command": command}
        save()
        print(f"Starting {job['key']}: {len(job['cases'])} clients, {job['timeout']}s program bound", flush=True)
        with (job_dir / "harness.log").open("w") as output:
            child = subprocess.Popen(command, cwd=root, env=env, stdout=output, stderr=subprocess.STDOUT)
            interval = 10
            previous_failures = set()
            while child.poll() is None:
                try:
                    child.wait(timeout=interval)
                except subprocess.TimeoutExpired:
                    pass
                report = assess(read_results(result_path), job, manifest["sources"], artifacts["prusti-driver"])
                failures = set(report["failures"])
                fresh_failures = failures - previous_failures
                previous_failures = failures
                interval = 10 if fresh_failures else ({10: 30, 30: 60, 60: 300}.get(interval, min(interval * 2, 3600)))
                status["active"].update(report, next_monitor_seconds=interval, pid=child.pid)
                save()
                if fresh_failures:
                    print(f"Failures in {job['key']}: {', '.join(sorted(fresh_failures))}", flush=True)
        report = assess(read_results(result_path), job, manifest["sources"], artifacts["prusti-driver"])
        status["completed"].append({"job": job["key"], "exit_code": child.returncode, **report})
        status["active"] = None
        save()
        print(f"Finished {job['key']}: {report['passed']}/{report['checks']} checks, complete={report['complete']}", flush=True)
    check_sources()
    status["state"] = "complete" if all(
        job["complete"] and not job["failures"] for job in status["completed"]
    ) else "failed"
    save()
    print(f"Matrix {status['state']}: {len(status['completed'])} jobs; {log_dir / 'status.json'}", flush=True)
    if status["state"] != "complete":
        raise SystemExit(1)


if __name__ == "__main__":
    main()
