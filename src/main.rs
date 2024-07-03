use anyhow::{anyhow, Result};
use clap::{builder::PossibleValue, Parser, ValueEnum};
use kseq::parse_reader;
use regex::RegexBuilder;
use std::{
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    #[arg(short, long, default_value = "false")]
    pub debug: bool,
}

#[derive(Parser, Debug)]
pub enum Command {
    /// Search for sequences matching a pattern
    #[clap(alias = "gr")]
    Grep(GrepArgs),

    /// Count
    #[clap(alias = "co")]
    Count(CountArgs),
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct CountArgs {
    /// Input file(s)
    #[arg(value_name = "FILE", default_value = "-")]
    files: Vec<String>,
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct GrepArgs {
    /// Output format
    #[arg(
        short('f'),
        long,
        value_name = "OUTFMT", 
        value_parser(clap::value_parser!(OutputFormat))
    )]
    outfmt: Option<OutputFormat>,

    /// Output file
    #[arg(short, long, value_name = "OUTPUT")]
    output: Option<String>,

    /// Search record part
    #[arg(
        short,
        long,
        value_name = "PART",
        default_value = "head",
        value_parser(clap::value_parser!(GrepRecordPart))
    )]
    part: Option<GrepRecordPart>,

    /// Invert match
    #[arg(short('v'), long("invert-match"))]
    invert: bool,

    /// Case-insensitive search
    #[arg(short('i'), long, value_name = "INSENSITIVE")]
    insensitive: bool,

    /// Pattern
    #[arg(value_name = "PATTERN")]
    pattern: String,

    /// Input file(s)
    #[arg(value_name = "FILE", default_value = "-")]
    files: Vec<String>,
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum GrepRecordPart {
    Head,
    Sequence,
    Quality,
}

impl ValueEnum for GrepRecordPart {
    fn value_variants<'a>() -> &'a [Self] {
        &[
            GrepRecordPart::Head,
            GrepRecordPart::Sequence,
            GrepRecordPart::Quality,
        ]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(match self {
            GrepRecordPart::Head => PossibleValue::new("head"),
            GrepRecordPart::Sequence => PossibleValue::new("seq"),
            GrepRecordPart::Quality => PossibleValue::new("qual"),
        })
    }
}

#[derive(Debug, Eq, PartialEq, Clone)]
enum OutputFormat {
    Fasta,
    Fastq,
}

impl ValueEnum for OutputFormat {
    fn value_variants<'a>() -> &'a [Self] {
        &[OutputFormat::Fasta, OutputFormat::Fastq]
    }

    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
        Some(match self {
            OutputFormat::Fasta => PossibleValue::new("fasta"),
            OutputFormat::Fastq => PossibleValue::new("fastq"),
        })
    }
}

// --------------------------------------------------
fn main() {
    if let Err(e) = run(Cli::parse()) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

// --------------------------------------------------
fn run(args: Cli) -> Result<()> {
    match &args.command {
        Some(Command::Grep(args)) => {
            grep(args.clone())?;
            Ok(())
        }
        Some(Command::Count(args)) => {
            count(args.clone())?;
            Ok(())
        }
        _ => unreachable!(),
    }
}

// --------------------------------------------------
fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

// --------------------------------------------------
fn count(args: CountArgs) -> Result<()> {
    for filename in &args.files {
        match open(filename) {
            Err(e) => eprintln!("{filename}: {e}"),
            Ok(file) => {
                let mut reader = parse_reader(file)?;
                let mut num = 0;
                while let Some(_) = reader.iter_record()? {
                    num += 1;
                }

                if filename == "-" {
                    println!("{num}");
                } else {
                    println!("{filename}: {num}");
                }
            }
        }
    }
    Ok(())
}

// --------------------------------------------------
fn grep(args: GrepArgs) -> Result<()> {
    let pattern = RegexBuilder::new(&args.pattern)
        .case_insensitive(args.insensitive)
        .build()
        .map_err(|_| anyhow!(r#"Invalid pattern "{}""#, args.pattern))?;

    let mut output: Box<dyn Write> = match &args.output {
        Some(out_name) => Box::new(File::create(out_name)?),
        _ => Box::new(io::stdout()),
    };

    for filename in &args.files {
        match open(filename) {
            Err(e) => eprintln!("{filename}: {e}"),
            Ok(file) => {
                let mut reader = parse_reader(file)?;
                let mut outfmt = &args.outfmt;
                while let Some(rec) = reader.iter_record()? {
                    let search = match &args.part {
                        Some(GrepRecordPart::Head) => {
                            format!("{}{}", rec.head(), rec.des())
                        }
                        Some(GrepRecordPart::Sequence) => {
                            rec.seq().to_string()
                        }
                        Some(GrepRecordPart::Quality) => {
                            rec.qual().to_string()
                        }
                        _ => unreachable!(),
                    };

                    if pattern.is_match(&search) ^ args.invert {
                        if outfmt.is_none() {
                            outfmt = if rec.qual().len() > 0 {
                                &Some(OutputFormat::Fastq)
                            } else {
                                &Some(OutputFormat::Fasta)
                            };
                        }

                        match outfmt {
                            Some(OutputFormat::Fasta) => {
                                writeln!(
                                    output,
                                    ">{}{}\n{}",
                                    rec.head(),
                                    rec.des(),
                                    rec.seq()
                                )?;
                            }
                            Some(OutputFormat::Fastq) => {
                                writeln!(
                                    output,
                                    "@{}{}\n{}\n{}\n{}",
                                    rec.head(),
                                    rec.des(),
                                    rec.seq(),
                                    if rec.sep().is_empty() {
                                        "+"
                                    } else {
                                        rec.sep()
                                    },
                                    if rec.qual().is_empty() {
                                        "-".repeat(rec.seq().len())
                                    } else {
                                        rec.qual().to_string()
                                    },
                                )?;
                            }
                            _ => unreachable!(),
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
