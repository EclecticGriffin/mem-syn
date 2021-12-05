mod dsl;
mod solver;
mod structures;

use dsl::ast::AstParser;
use dsl::Trace;

use argh::FromArgs;
use std::fs::File;
use std::io::prelude::*;
use std::io::Write;
use std::path::Path;

#[derive(FromArgs)]
/// Synthesize a memory from input traces or generate a calyx implementation
struct Args {
    #[argh(subcommand)]
    command: Command,

    #[argh(option, short = 'o')]
    /// file to write to
    output: Option<String>,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Command {
    Synthesize(SynthesizeCommand),
    Output(OutputCommand),
    Verify(VerifyCommand),
}

#[derive(FromArgs, PartialEq, Debug)]
/// Synthesize a memory implementation from a trace
#[argh(subcommand, name = "synthesize")]
struct SynthesizeCommand {
    /// file to read the trace from
    #[argh(positional)]
    trace_file: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Output Calyx implementation for the given description
#[argh(subcommand, name = "emit")]
struct OutputCommand {
    /// file to read the description from
    #[argh(positional)]
    memory_description: String,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Verify that a given description satisfies the trace
#[argh(subcommand, name = "verify")]
struct VerifyCommand {
    /// file to read the description from
    #[argh(positional)]
    memory_description: String,

    /// file to read the trace from
    #[argh(positional)]
    trace_file: String,
}

fn main() {
    let args: Args = argh::from_env();

    let mut output: Box<dyn Write> = args.output.map_or_else(
        || Box::new(std::io::stdout()) as Box<dyn Write>,
        |x| Box::new(File::create(&Path::new(&x)).unwrap()),
    );

    match args.command {
        Command::Synthesize(s) => {
            let mut file = File::open(&Path::new(&s.trace_file)).expect("Couldn't find trace file");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Couldn't read trace file");

            let trace = Trace::parse_trace(contents).expect("malformed trace file");

            let comp = solver::solve_trace(&trace);
            write!(output, "{}", comp.pretty_print()).unwrap();
        }
        Command::Output(OutputCommand { memory_description }) => {
            let mut file = File::open(&Path::new(&memory_description))
                .expect("Couldn't find description file");
            let mut contents = String::new();
            file.read_to_string(&mut contents)
                .expect("Couldn't read trace file");
            let description =
                AstParser::parse_component(contents).expect("Couldn't parse description");
            write!(output, "{}", description.emit_calyx_comp()).unwrap();
        }
        Command::Verify(VerifyCommand {
            memory_description,
            trace_file,
        }) => {
            let mut description_file = File::open(&Path::new(&memory_description))
                .expect("Couldn't find description file");
            let mut description = String::new();
            description_file
                .read_to_string(&mut description)
                .expect("Couldn't read description file");

            let mut trace_file =
                File::open(&Path::new(&trace_file)).expect("Couldn't find trace file");
            let mut trace = String::new();
            trace_file
                .read_to_string(&mut trace)
                .expect("Couldn't read trace file");

            let trace = Trace::parse_trace(trace).expect("malformed trace file");
            let comp = AstParser::parse_component(description).expect("Couldn't parse description");
            let result = comp.vailidate(&trace);

            if result {
                println!("✅ Validated successfully")
            } else {
                println!("❌ Validation failed")
            }
        }
    }
}
