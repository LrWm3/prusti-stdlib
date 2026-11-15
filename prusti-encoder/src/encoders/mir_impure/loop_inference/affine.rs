//! Bounded, untrusted candidate inference over an immutable integer loop model.
//! Every returned fact must still be checked as an ordinary native invariant.
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Candidate {
    /// Coefficients of counter, remaining length, original length.
    pub coefficients: [i32; 3],
    pub equality: bool,
}
impl Candidate {
    fn smt(self, next: bool) -> String {
        let names = if next {
            ["s1", "r1", "n1"]
        } else {
            ["s", "r", "n"]
        };
        let terms = self
            .coefficients
            .iter()
            .zip(names)
            .filter(|(c, _)| **c != 0)
            .map(|(c, n)| format!("(* {c} {n})"))
            .collect::<Vec<_>>()
            .join(" ");
        format!("({} (+ {terms}) 0)", if self.equality { "=" } else { ">=" })
    }
}

/// Only constructed after extracting a single finite-iterator step and the
/// matching MIR counter update. Frames and permissions are outside this model.
pub(crate) struct CountLoop {
    pub counter_delta: i64,
}
pub(crate) struct Inference {
    pub survivors: Vec<Candidate>,
    pub generated: usize,
    pub queries: usize,
    pub elapsed: Duration,
    pub transcript: String,
}
struct Solver {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    queries: usize,
    transcript: String,
}
impl Drop for Solver {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
impl Solver {
    fn new(path: &str) -> Result<Self, String> {
        let mut child = Command::new(path)
            .args(["-in", "-smt2", "-T:5"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut result = Self {
            child,
            input,
            output,
            queries: 0,
            transcript: String::new(),
        };
        result.send("(set-option :timeout 100)\n(set-option :smt.random_seed 0)\n")?;
        for name in ["s", "r", "n", "s1", "r1", "n1"] {
            result.send(&format!("(declare-const {name} Int)\n"))?;
        }
        Ok(result)
    }
    fn send(&mut self, command: &str) -> Result<(), String> {
        self.transcript.push_str(command);
        self.input
            .write_all(command.as_bytes())
            .map_err(|e| e.to_string())?;
        self.input.flush().map_err(|e| e.to_string())
    }
    fn proves(&mut self, context: &str, goal: &str) -> Result<bool, String> {
        if self.queries >= 200 {
            return Err("local inference query budget exhausted".into());
        }
        self.queries += 1;
        self.send(&format!(
            "(push 1)\n(assert {context})\n(assert (not {goal}))\n(check-sat)\n"
        ))?;
        let mut answer = String::new();
        self.output
            .read_line(&mut answer)
            .map_err(|e| e.to_string())?;
        self.transcript
            .push_str(&format!("; result: {}\n", answer.trim()));
        self.send("(pop 1)\n")?;
        match answer.trim() {
            "unsat" => Ok(true),
            "sat" | "unknown" => Ok(false),
            other => Err(format!("local inference solver stopped: {other}")),
        }
    }
}

pub(crate) fn infer(model: &CountLoop, solver: &str) -> Result<Inference, String> {
    let started = Instant::now();
    let mut smt = Solver::new(solver)?;
    let mut candidates = Vec::new();
    // A small affine vocabulary: no user-variable-name or postcondition match.
    for equality in [true, false] {
        for s in -1..=1 {
            for r in -1..=1 {
                for n in -1..=1 {
                    let c = [s, r, n];
                    let Some(first) = c.iter().find(|c| **c != 0) else {
                        continue;
                    };
                    if equality && *first < 0 {
                        continue;
                    }
                    candidates.push(Candidate {
                        coefficients: c,
                        equality,
                    });
                }
            }
        }
    }
    let generated = candidates.len();
    let mut entry = Vec::new();
    for candidate in candidates {
        if smt.proves("(and (= s 0) (= r n) (>= n 0))", &candidate.smt(false))? {
            entry.push(candidate);
        }
    }
    let transition = format!(
        "(and (> r 0) (= s1 (+ s {})) (= r1 (- r 1)) (= n1 n))",
        model.counter_delta
    );
    loop {
        let context = format!(
            "(and {transition} {})",
            entry
                .iter()
                .map(|c| c.smt(false))
                .collect::<Vec<_>>()
                .join(" ")
        );
        let mut keep = Vec::new();
        for candidate in &entry {
            if smt.proves(&context, &candidate.smt(true))? {
                keep.push(*candidate);
            }
        }
        if keep == entry {
            break;
        }
        entry = keep;
    }
    // Avoid flooding the verifier with redundant inferred guarantees.
    let mut minimal = Vec::<Candidate>::new();
    for candidate in entry {
        let context = format!(
            "(and true {})",
            minimal
                .iter()
                .map(|c| c.smt(false))
                .collect::<Vec<_>>()
                .join(" ")
        );
        if !smt.proves(&context, &candidate.smt(false))? {
            minimal.push(candidate);
        }
    }
    Ok(Inference {
        survivors: minimal,
        generated,
        queries: smt.queries,
        elapsed: started.elapsed(),
        transcript: std::mem::take(&mut smt.transcript),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn count_and_wrong_updates_have_different_inductive_relations() {
        let solver = std::env::var("Z3_EXE").expect("pinned Z3 required");
        let conservation = Candidate {
            coefficients: [1, 1, -1],
            equality: true,
        };
        let good = infer(&CountLoop { counter_delta: 1 }, &solver).unwrap();
        assert!(good.survivors.contains(&conservation));
        assert!(good.queries <= 200);
        for delta in [0, 2] {
            let bad = infer(
                &CountLoop {
                    counter_delta: delta,
                },
                &solver,
            )
            .unwrap();
            assert!(!bad.survivors.contains(&conservation));
        }
    }
}
