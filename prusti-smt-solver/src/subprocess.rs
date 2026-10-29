use crate::context::Context;
use async_std::{
    io::{prelude::BufReadExt, BufReader, WriteExt},
    process::{ChildStderr, ChildStdin, ChildStdout},
};

// SMT-LIB replies can be multiline strings as well as S-expressions. Delimiters
// inside quoted values or comments do not end a command or response.
fn are_parens_balanced(input: &str) -> bool {
    let mut depth = 0_i64;
    let mut in_string = false;
    let mut in_symbol = false;
    let mut in_comment = false;
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if in_comment {
            if ch == '\n' {
                in_comment = false;
            }
        } else if in_string {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_string = false;
                }
            }
        } else if in_symbol {
            if ch == '|' {
                in_symbol = false;
            } else if ch == '\\' {
                chars.next();
            }
        } else {
            match ch {
                ';' => in_comment = true,
                '"' => in_string = true,
                '|' => in_symbol = true,
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => (),
            }
        }
    }
    depth == 0 && !in_string && !in_symbol
}

async fn read_command(command: &mut String) -> Result<bool, std::io::Error> {
    command.clear();
    let stdin = async_std::io::stdin();
    while stdin.read_line(command).await? > 0 {
        if are_parens_balanced(command) {
            return Ok(true);
        }
    }
    Ok(false)
}

async fn read_response(
    solver_stdout: &mut BufReader<ChildStdout>,
    response: &mut String,
) -> Result<bool, std::io::Error> {
    response.clear();
    while solver_stdout.read_line(response).await? > 0 {
        if are_parens_balanced(response) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) async fn communicate(
    context: &Context,
    mut solver_stdin: ChildStdin,
    solver_stdout: ChildStdout,
) -> Result<(), std::io::Error> {
    let mut stdout = async_std::io::stdout();
    let mut solver_stdout = BufReader::new(solver_stdout);
    let mut response = String::new();
    let mut command = String::new();
    let mut not_finished = true;
    while not_finished && read_command(&mut command).await? {
        context.write_to_log("in ", &command).await?;
        let now = std::time::Instant::now();
        solver_stdin.write_all(command.as_bytes()).await?;
        solver_stdin.flush().await?;

        if !read_response(&mut solver_stdout, &mut response)
            .await
            .unwrap()
        {
            assert!(
                are_parens_balanced(&response),
                "unbalanced EOF response: {response}"
            );
            not_finished = false;
        }
        let elapsed = now.elapsed().as_millis();
        context.write_to_log("out", &response).await?;
        context.write_number_to_log("elapsed-time", elapsed).await?;
        stdout.write_all(response.as_bytes()).await?;
        stdout.flush().await?;

        context.check(&command, &response).await?;
    }

    Ok(())
}

pub(crate) async fn get_version(
    context: &Context,
    solver_stdout: ChildStdout,
) -> Result<(), std::io::Error> {
    let mut stdout = async_std::io::stdout();
    let mut solver_stdout = BufReader::new(solver_stdout);
    let mut response = String::new();
    while solver_stdout.read_line(&mut response).await? > 0 {}
    context.write_to_log("out", &response).await?;
    stdout.write_all(response.as_bytes()).await?;
    stdout.flush().await?;
    Ok(())
}

pub(crate) async fn pass_error(
    context: &Context,
    solver_stderr: ChildStderr,
) -> Result<(), std::io::Error> {
    let mut solver_stderr = BufReader::new(solver_stderr);
    let mut stderr = async_std::io::stderr();
    let mut line = String::new();
    context.write_to_log("err", "stderr started\n").await?;
    while solver_stderr.read_line(&mut line).await? > 0 {
        context.write_to_log("err", &line).await?;
        stderr.write_all(line.as_bytes()).await?;
        stderr.flush().await?;
        line.clear();
    }
    context.write_to_log("err", "stderr finished\n").await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::are_parens_balanced;
    #[test]
    fn complete_responses_respect_smt_quoted_values() {
        assert!(!are_parens_balanced(
            "\"snapshot -> ($Snap.combine $Snap.unit $Snap.unit)\n"
        ));
        assert!(are_parens_balanced(
            "\"snapshot -> ($Snap.combine $Snap.unit $Snap.unit)\nnext value\"\n"
        ));
        assert!(are_parens_balanced(
            "(assert (= |a(b| \"x)\")) ; ( ignored\n"
        ));
        assert!(are_parens_balanced("(echo \"embedded \"\" quote\")"));
        assert!(are_parens_balanced("(echo \"literal-backslash\\\")"));
        assert!(!are_parens_balanced("(declare-const |escaped\\| Int)"));
        assert!(!are_parens_balanced("(assert (= |unfinished ("));
    }
}
