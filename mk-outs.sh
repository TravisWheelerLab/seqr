#!/usr/bin/env bash

set -u

GREP="cargo run -- grep"
DFAM="tests/inputs/dfam.fa"
OUTDIR="tests/outputs"

[[ ! -d "$OUTDIR" ]] && mkdir -p "$OUTDIR"

$GREP Alu          "$DFAM" > "$OUTDIR/dfam.alu.fa"
$GREP Alu -f fastq "$DFAM" > "$OUTDIR/dfam.alu.fq"
$GREP Alu -i       "$DFAM" > "$OUTDIR/dfam.alu.insensitive.fa"
