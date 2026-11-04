//! Guide-run and error-sink tests (split from tests.rs for the pretender
//! file-lines budget).

use super::*;

// -- Guide run ---------------------------------------------------------

#[test]
fn test_guide_new_sets_name_and_version() {
    let guide = Guide::builder("my-tool", "1.0.0").build();
    assert_eq!(guide.name(), "my-tool");
    assert_eq!(guide.version(), "1.0.0");
}

#[test]
fn test_guide_with_commands() {
    let guide = Guide::builder("tool", "0.1")
        .commands(&["init", "check", "doctor"])
        .build();

    let all = guide.registry().all();
    assert_eq!(all.len(), 3);
    assert!(all.contains(&"init"));
    assert!(all.contains(&"check"));
    assert!(all.contains(&"doctor"));
}

#[test]
fn test_guide_with_verbosity() {
    let guide = Guide::builder("tool", "0.1").max_verbosity(2).build();
    assert_eq!(guide.verbosity(), Verbosity::Verbose);
}

#[test]
fn test_guide_run_success_returns_zero() {
    let guide = Guide::builder("test", "0.1").build();
    let exit: i32 = guide
        .run(|| -> Result<Output<&str>, Box<dyn std::error::Error>> { Ok(Output::success("ok")) });
    assert_eq!(exit, 0);
}

#[test]
fn test_guide_run_failure_returns_one() {
    let guide = Guide::builder("test", "0.1").build();
    let exit: i32 = guide.run(|| -> Result<Output<String>, Box<dyn std::error::Error>> {
        let output: Output<String> = Output::failure("nope");
        Ok(output)
    });
    assert_eq!(exit, 1);
}

#[test]
fn test_guide_run_error_returns_one() {
    let guide = Guide::builder("test", "0.1").build();
    let exit: i32 = guide.run(|| -> Result<Output<String>, Box<dyn std::error::Error>> {
        Err("something went wrong".into())
    });
    assert_eq!(exit, 1);
}

/// Always-failing writer used to exercise the internal-failure path.
struct FailingWriter;

impl std::io::Write for FailingWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("simulated I/O failure"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_guide_run_print_failure_returns_two() {
    let guide = Guide::builder("test", "0.1").build();
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();
    let exit: i32 = guide.run_with_writers(
        || -> Result<Output<&str>, Box<dyn std::error::Error>> { Ok(Output::success("ok")) },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 2);
}

#[test]
fn test_guide_run_formatted_print_failure_returns_two() {
    let guide = Guide::builder("test", "0.1").build();
    let mut stdout = FailingWriter;
    let mut stderr = Vec::new();
    let exit: i32 = guide.run_formatted_with_writers(
        OutputFormat::Json,
        || -> Result<Output<&str>, Box<dyn std::error::Error>> { Ok(Output::success("ok")) },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 2);
}

#[test]
fn test_guide_run_with_writers_user_error_still_returns_one() {
    let guide = Guide::builder("test", "0.1").build();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let exit: i32 = guide.run_with_writers(
        || -> Result<Output<String>, Box<dyn std::error::Error>> { Ok(Output::failure("nope")) },
        &mut stdout,
        &mut stderr,
    );
    assert_eq!(exit, 1);
}

#[test]
fn test_guide_error_sink_is_configured() {
    let guide = Guide::builder("my-tool", "0.1").build();
    let sink = guide.error_sink();
    assert_eq!(sink.tool_name, "my-tool");
}
