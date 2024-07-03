use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use pretty_assertions::assert_eq;
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use std::fs;
use tempfile::NamedTempFile;

const PRG: &str = "seqr";
const DFAM: &str = "tests/inputs/dfam.fa";
const OUT_DFAM_ALU_FA: &str = "tests/outputs/dfam.alu.fa";
const OUT_DFAM_ALU_FQ: &str = "tests/outputs/dfam.alu.fq";
const OUT_DFAM_ALU_I_FA: &str = "tests/outputs/dfam.alu.insensitive.fa";

// --------------------------------------------------
fn gen_nonexistent_file() -> String {
    loop {
        let filename: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        if fs::metadata(&filename).is_err() {
            return filename;
        }
    }
}

// --------------------------------------------------
#[test]
fn dies_no_args() -> Result<()> {
    Command::cargo_bin(PRG)?
        .assert()
        .failure()
        .stderr(predicate::str::contains("Usage"));
    Ok(())
}

// --------------------------------------------------
#[test]
fn grep_dies_bad_pattern() -> Result<()> {
    Command::cargo_bin(PRG)?
        .args(["grep", "*foo"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(r#"Invalid pattern "*foo""#));
    Ok(())
}

// --------------------------------------------------
#[test]
fn grep_warns_bad_file() -> Result<()> {
    let bad = gen_nonexistent_file();
    let expected = format!("{bad}: .* [(]os error 2[)]");
    Command::cargo_bin(PRG)?
        .args(["grep", "foo", &bad, DFAM])
        .assert()
        .stderr(predicate::str::is_match(expected)?);
    Ok(())
}

// --------------------------------------------------
fn run_stdout(args: &[&str], expected_file: &str) -> Result<()> {
    let expected = fs::read_to_string(expected_file)?;
    let output = Command::cargo_bin(PRG)?.args(args).output().expect("fail");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    assert_eq!(stdout, expected);

    Ok(())
}

// --------------------------------------------------
fn run_stdin(
    input_file: &str,
    args: &[&str],
    expected_file: &str,
) -> Result<()> {
    let input = fs::read_to_string(input_file)?;
    let expected = fs::read_to_string(expected_file)?;
    let output = Command::cargo_bin(PRG)?
        .write_stdin(input)
        .args(args)
        .output()
        .expect("fail");
    assert!(output.status.success());

    let stdout = String::from_utf8(output.stdout).expect("invalid UTF-8");
    assert_eq!(stdout, expected);

    Ok(())
}

// --------------------------------------------------
fn run_outfile(args: &[&str], expected_file: &str) -> Result<()> {
    let outfile = NamedTempFile::new()?;
    let outpath = &outfile.path().to_str().unwrap();
    let mut args: Vec<_> = args.into_iter().map(|v| v.to_string()).collect();
    args.extend_from_slice(&["-o".to_string(), outpath.to_string()]);

    Command::cargo_bin(PRG)?
        .args(args)
        .assert()
        .success()
        .stdout("");

    let expected = fs::read_to_string(expected_file)?;
    let contents = fs::read_to_string(outpath)?;
    assert_eq!(&expected, &contents);

    Ok(())
}

// --------------------------------------------------
#[test]
fn grep_dies_bad_outfmt() -> Result<()> {
    let output = Command::cargo_bin(PRG)?
        .args(&["grep", "-f", "fastp", "Alu", DFAM])
        .output()?;

    let stderr = String::from_utf8(output.stderr)?;
    let re = Regex::new("error: invalid value 'fastp' for '--outfmt")?;
    assert!(re.is_match(&stderr));

    Ok(())
}

// --------------------------------------------------
#[test]
fn grep_alu_stdout() -> Result<()> {
    run_stdout(&["grep", "Alu", DFAM], OUT_DFAM_ALU_FA)
}

// --------------------------------------------------
#[test]
fn grep_alu_insensitive_stdout() -> Result<()> {
    run_stdout(&["grep", "-i", "alu", DFAM], OUT_DFAM_ALU_I_FA)
}

// --------------------------------------------------
#[test]
fn grep_alu_fastq_stdout() -> Result<()> {
    run_stdout(&["grep", "-f", "fastq", "Alu", DFAM], OUT_DFAM_ALU_FQ)
}

// --------------------------------------------------
#[test]
fn grep_alu_stdin() -> Result<()> {
    run_stdin(DFAM, &["grep", "Alu"], OUT_DFAM_ALU_FA)
}

// --------------------------------------------------
#[test]
fn grep_alu_outfile() -> Result<()> {
    run_outfile(&["grep", "Alu", DFAM], OUT_DFAM_ALU_FA)
}
