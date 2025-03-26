use anyhow::{anyhow, bail, Result};
use clap::{builder::PossibleValue, Parser, ValueEnum};
use kseq::{parse_reader, record::Fastx};
use regex::RegexBuilder;
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    io::{self, BufRead, BufReader, Write},
};

#[derive(Parser, Debug)]
#[command(arg_required_else_help = true, version)]
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

    /// Show headers
    #[clap(alias = "he")]
    Headers(HeadersArgs),

    /// File statistics
    #[clap(alias = "st")]
    Stats(StatsArgs),

    /// Filter
    #[clap(alias = "fi")]
    Filter(FilterArgs),
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
pub struct HeadersArgs {
    /// Input file(s)
    #[arg(value_name = "FILE", default_value = "-")]
    files: Vec<String>,

    /// Print ID only
    #[arg(short, long("id"))]
    id_only: bool,

    /// Print description only
    #[arg(short, long("desc"))]
    desc_only: bool,
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct StatsArgs {
    /// Input file(s)
    #[arg(value_name = "FILE", default_value = "-")]
    file: String,

    /// Top N by length
    #[arg(short, long("top-n"), value_name = "TOP_N", default_value = "100")]
    top_n: usize,
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct FilterArgs {
    /// Minimum sequence length
    #[arg(short, long("min-len"), value_name = "LEN", default_value = "0")]
    min_length: usize,

    /// Maximum sequence length
    #[arg(short('x'), long("max-len"), value_name = "LEN", default_value = "0")]
    max_length: usize,

    /// Maximum number of sequences
    #[arg(short, long, value_name = "NUM", default_value = "0")]
    number: usize,

    /// Sequence IDs/headers
    #[arg(short, long, value_name = "IDS", num_args = 0..)]
    ids: Vec<String>,

    /// Sequence IDs/headers from file
    #[arg(short('f'), long, value_name = "FILE")]
    ids_from_file: Option<String>,

    /// Output
    #[arg(short, long, value_name = "OUT")]
    output: Option<String>,

    /// Input file(s)
    #[arg(value_name = "FILE", default_value = "-")]
    file: String,
}

#[derive(Debug, Clone, Parser)]
#[command(author, version, about)]
pub struct GrepArgs {
    ///// Output format
    //#[arg(
    //    short('f'),
    //    long,
    //    value_name = "OUTFMT",
    //    value_parser(clap::value_parser!(OutputFormat))
    //)]
    //outfmt: Option<OutputFormat>,
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

//#[derive(Debug, Eq, PartialEq, Clone)]
//enum OutputFormat {
//    Fasta,
//    Fastq,
//}

//impl ValueEnum for OutputFormat {
//    fn value_variants<'a>() -> &'a [Self] {
//        &[OutputFormat::Fasta, OutputFormat::Fastq]
//    }

//    fn to_possible_value<'a>(&self) -> Option<PossibleValue> {
//        Some(match self {
//            OutputFormat::Fasta => PossibleValue::new("fasta"),
//            OutputFormat::Fastq => PossibleValue::new("fastq"),
//        })
//    }
//}

// --------------------------------------------------
//fn main() {
//    if let Err(e) = run(Cli::parse()) {
//        eprintln!("{e}");
//        std::process::exit(1);
//    }
//}

// --------------------------------------------------
pub fn run(args: Cli) -> Result<()> {
    match &args.command {
        Some(Command::Count(args)) => {
            count(args.clone())?;
            Ok(())
        }
        Some(Command::Grep(args)) => {
            grep(args.clone())?;
            Ok(())
        }
        Some(Command::Headers(args)) => {
            headers(args.clone())?;
            Ok(())
        }
        Some(Command::Filter(args)) => {
            filter(args.clone())?;
            Ok(())
        }
        Some(Command::Stats(args)) => {
            stats(args.clone())?;
            Ok(())
        }
        _ => unreachable!(),
    }
}

// --------------------------------------------------
fn open(filename: &str) -> Result<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            File::open(filename).map_err(|e| anyhow!("{filename}: {e}"))?,
        ))),
    }
}

// --------------------------------------------------
fn read_lines(path: &str) -> Result<Vec<String>> {
    Ok(open(path)?
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.is_empty())
        .collect())
}

// --------------------------------------------------
fn filter(args: FilterArgs) -> Result<()> {
    let mut reader =
        parse_reader(open(&args.file).map_err(|e| anyhow!("{}: {e}", args.file))?)?;

    let mut output: Box<dyn Write> = match &args.output {
        Some(out_name) => Box::new(File::create(out_name)?),
        _ => Box::new(io::stdout()),
    };

    let ids = if let Some(filename) = args.ids_from_file {
        read_lines(&filename)?
    } else {
        args.ids
    };
    let id_lookup = HashSet::<String>::from_iter(ids);
    let id_filter = |rec: &Fastx| -> bool {
        id_lookup.is_empty()
            || id_lookup.contains(rec.head())
            || id_lookup.contains(rec.des())
    };
    let max_len_filter = |rec: &Fastx| -> bool {
        args.max_length == 0 || (rec.seq().len() <= args.max_length)
    };
    let min_len_filter = |rec: &Fastx| -> bool {
        args.min_length == 0 || (rec.seq().len() >= args.min_length)
    };
    let mut num_taken = 0;

    while let Some(rec) = reader.iter_record()? {
        if !max_len_filter(&rec) || !min_len_filter(&rec) || !id_filter(&rec) {
            continue;
        }

        if rec.is_fasta() {
            writeln!(output, ">{}{}\n{}", rec.head(), rec.des(), rec.seq())?;
        } else {
            writeln!(
                output,
                "@{}{}\n{}\n{}\n{}",
                rec.head(),
                rec.des(),
                rec.seq(),
                if rec.sep().is_empty() { "+" } else { rec.sep() },
                if rec.qual().is_empty() {
                    "-".repeat(rec.seq().len())
                } else {
                    rec.qual().to_string()
                },
            )?;
        }

        num_taken += 1;
        if !args.number == 0 || num_taken == args.number {
            break;
        }
    }

    Ok(())
}

// --------------------------------------------------
fn stats(args: StatsArgs) -> Result<()> {
    let mut reader =
        parse_reader(open(&args.file).map_err(|e| anyhow!("{}: {e}", args.file))?)?;
    let mut num_by_len: HashMap<usize, usize> = HashMap::new();
    let mut avg: i64 = 0;
    let mut counter = 0;

    while let Some(rec) = reader.iter_record()? {
        let len = rec.seq().len();
        println!("= {}\t{len}", rec.head());

        // Cf. https://en.wikipedia.org/wiki/Moving_average
        counter += 1;
        avg = avg + ((len as i64 - avg) / counter);

        if let Some(val) = num_by_len.get_mut(&len) {
            *val += 1;
        } else {
            num_by_len.insert(len, 1);
        }
    }

    let num_seqs: usize = num_by_len.values().sum();

    if num_seqs > 0 {
        let mut lengths: Vec<&usize> = num_by_len.keys().collect();
        lengths.sort();
        lengths.reverse();
        println!("Num seqs: {num_seqs}");
        println!("Smallest: {:?}", lengths.last().unwrap_or(&&0));
        println!("Largest: {:?}", lengths.first().unwrap_or(&&0));
        println!("Average: {avg}");

        // Accumulate the number of sequences by descending
        // order of lengths and stop when we've found the top N
        let mut top_n = 0;
        for len in lengths {
            top_n += num_by_len.get(&len).unwrap_or(&0);

            if top_n >= args.top_n {
                println!("Top {}: {:?}", args.top_n, len);
                break;
            }
        }
    } else {
        bail!("No sequences found!");
    }

    Ok(())
}

// --------------------------------------------------
fn count(args: CountArgs) -> Result<()> {
    let num_files = args.files.len();
    let mut total = 0;

    for filename in &args.files {
        match open(filename) {
            Err(e) => eprintln!("{filename}: {e}"),
            Ok(file) => {
                let mut reader = parse_reader(file)?;
                let mut num = 0;
                while (reader.iter_record()?).is_some() {
                    num += 1;
                }

                if filename == "-" {
                    println!("{num:>10}");
                } else {
                    println!("{num:>10} {filename}");
                }
                total += num;
            }
        }
    }

    if num_files > 1 {
        println!("{total:>10}: total");
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
                //let mut outfmt = &args.outfmt;
                while let Some(rec) = reader.iter_record()? {
                    let search = match &args.part {
                        Some(GrepRecordPart::Head) => {
                            format!("{}{}", rec.head(), rec.des())
                        }
                        Some(GrepRecordPart::Sequence) => rec.seq().to_string(),
                        Some(GrepRecordPart::Quality) => rec.qual().to_string(),
                        _ => unreachable!(),
                    };

                    if pattern.is_match(&search) ^ args.invert {
                        if rec.is_fasta() {
                            writeln!(
                                output,
                                ">{}{}\n{}",
                                rec.head(),
                                rec.des(),
                                rec.seq()
                            )?;
                        } else {
                            writeln!(
                                output,
                                "@{}{}\n{}\n{}\n{}",
                                rec.head(),
                                rec.des(),
                                rec.seq(),
                                if rec.sep().is_empty() { "+" } else { rec.sep() },
                                if rec.qual().is_empty() {
                                    "-".repeat(rec.seq().len())
                                } else {
                                    rec.qual().to_string()
                                },
                            )?;
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

// --------------------------------------------------
fn headers(args: HeadersArgs) -> Result<()> {
    for filename in &args.files {
        match open(filename) {
            Err(e) => eprintln!("{filename}: {e}"),
            Ok(file) => {
                let mut reader = parse_reader(file)?;
                while let Some(rec) = reader.iter_record()? {
                    if args.id_only {
                        println!("{}", rec.head());
                    } else if args.desc_only {
                        println!("{}", rec.des().trim());
                    } else {
                        println!("{}{}", rec.head(), rec.des());
                    }
                }
            }
        }
    }

    Ok(())
}

//fn write_seq(
//    rec: kseq::Fastx,
//    output: impl Write,
//) -> Result<()> {
//    match format {
//        Some(OutputFormat::Fasta) => {
//            writeln!(output, ">{}{}\n{}", rec.head(), rec.des(), rec.seq())?;
//        }
//        Some(OutputFormat::Fastq) => {
//            writeln!(
//                output,
//                "@{}{}\n{}\n{}\n{}",
//                rec.head(),
//                rec.des(),
//                rec.seq(),
//                if rec.sep().is_empty() { "+" } else { rec.sep() },
//                if rec.qual().is_empty() {
//                    "-".repeat(rec.seq().len())
//                } else {
//                    rec.qual().to_string()
//                },
//            )?;
//        }
//    }
//    Ok(())
//}
