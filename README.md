# seqr

A FASTX sequence tool written in Rust.

## Usage

```
Usage: seqr [OPTIONS] [COMMAND]

Commands:
  grep   Search for sequences matching a pattern
  count  Count
  help   Print this message or the help of the given subcommand(s)

Options:
  -d, --debug
  -h, --help   Print help
```

The `grep` command:

```
Usage: seqr grep [OPTIONS] <PATTERN> [FILE]...

Arguments:
  <PATTERN>  Pattern
  [FILE]...  Input file(s) [default: -]

Options:
  -f, --outfmt <OUTFMT>  Output format [possible values: fasta, fastq]
  -o, --output <OUTPUT>  Output file
  -p, --part <PART>      Search record part [default: head] 
                         [possible values: head, seq, qual]
  -v, --invert-match     Invert match
  -i, --insensitive      Case-insensitive search
  -h, --help             Print help
  -V, --version          Print version
```

The `count` command:

```
Usage: seqr count [FILE]...

Arguments:
  [FILE]...  Input file(s) [default: -]

Options:
  -h, --help     Print help
  -V, --version  Print version
```

## Author

Ken Youens-Clark <kyclark@arizona.edu>
