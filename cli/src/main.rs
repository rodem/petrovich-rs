//! `petrovich` CLI: thin wrapper over `petrovich-core`.
//!
//! ```text
//! petrovich decline --lastname Иванов --firstname Иван --case dative --gender male
//! petrovich gender --firstname Александра
//! petrovich decline --case instrumental --gender auto --batch names.tsv
//! petrovich appoint --appointment "генеральный директор" --case dative
//! petrovich appoint --appointment "Начальник цеха" --office "Цех нестандартного оборудования" --case genitive
//! ```
//!
//! Batch input is TSV `lastname<TAB>firstname<TAB>middlename` (empty fields
//! allowed); output mirrors it with inflected values. `--batch -` reads stdin.
//!
//! Exit codes: `0` success; `1` runtime error (I/O, empty name); `2` CLI usage
//! error (issued by clap itself).

use std::io::{BufRead, BufReader, Read};

use clap::{Parser, Subcommand, ValueEnum};
use petrovich::{Case, Gender, detect_gender};

#[derive(Parser)]
#[command(name = "petrovich", version, about = "Inflect Russian names")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Inflect a name into a grammatical case.
    Decline(DeclineArgs),
    /// Detect gender of a name.
    Gender(NameArgs),
    /// Inflect a job title, optionally merged with an office
    /// (first word declines, the rest stays as is).
    Appoint(AppointArgs),
}

#[derive(Parser)]
struct DeclineArgs {
    #[command(flatten)]
    name: NameArgs,
    /// Grammatical case.
    #[arg(long, value_enum, default_value_t = CaseArg::Genitive)]
    case: CaseArg,
    /// Gender; `auto` detects it from the given parts.
    #[arg(long, value_enum, default_value_t = GenderArg::Auto)]
    gender: GenderArg,
    /// Batch mode: TSV file (or `-` for stdin), one name per line.
    #[arg(long)]
    batch: Option<String>,
}

#[derive(Parser)]
struct AppointArgs {
    /// Job title, e.g. "генеральный директор".
    #[arg(long)]
    appointment: Option<String>,
    /// Office merged into the title (duplicates removed), e.g. "Цех ...".
    #[arg(long)]
    office: Option<String>,
    /// Grammatical case.
    #[arg(long, value_enum, default_value_t = CaseArg::Genitive)]
    case: CaseArg,
    /// Batch mode: TSV file (or `-` for stdin), `appointment<TAB>office` per line.
    #[arg(long)]
    batch: Option<String>,
}

#[derive(Parser)]
struct NameArgs {
    /// Last name.
    #[arg(long)]
    lastname: Option<String>,
    /// First name.
    #[arg(long)]
    firstname: Option<String>,
    /// Middle name.
    #[arg(long)]
    middlename: Option<String>,
}

#[derive(Copy, Clone, ValueEnum)]
enum CaseArg {
    Nominative,
    Genitive,
    Dative,
    Accusative,
    Instrumental,
    Prepositional,
}

impl CaseArg {
    fn case(self) -> Option<Case> {
        match self {
            CaseArg::Nominative => None,
            CaseArg::Genitive => Some(Case::Genitive),
            CaseArg::Dative => Some(Case::Dative),
            CaseArg::Accusative => Some(Case::Accusative),
            CaseArg::Instrumental => Some(Case::Instrumental),
            CaseArg::Prepositional => Some(Case::Prepositional),
        }
    }
}

#[derive(Copy, Clone, ValueEnum)]
enum GenderArg {
    Auto,
    Male,
    Female,
    Androgynous,
}

impl GenderArg {
    fn resolve(self, name: &NameArgs) -> Gender {
        match self {
            GenderArg::Auto => detect_gender(
                name.lastname.as_deref(),
                name.firstname.as_deref(),
                name.middlename.as_deref(),
            ),
            GenderArg::Male => Gender::Male,
            GenderArg::Female => Gender::Female,
            GenderArg::Androgynous => Gender::Androgynous,
        }
    }
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|s| !s.is_empty())
}

fn decline_parts(name: &NameArgs, gender: Gender, case: Case) -> Vec<String> {
    [
        (
            non_empty(&name.lastname),
            petrovich::lastname as fn(Gender, &str, Case) -> String,
        ),
        (non_empty(&name.firstname), petrovich::firstname),
        (non_empty(&name.middlename), petrovich::middlename),
    ]
    .into_iter()
    .filter_map(|(part, inflect)| part.map(|p| inflect(gender, p, case)))
    .collect()
}

fn decline_line(name: &NameArgs, gender: Gender, case: Option<Case>) -> Result<String, String> {
    if non_empty(&name.lastname).is_none()
        && non_empty(&name.firstname).is_none()
        && non_empty(&name.middlename).is_none()
    {
        return Err("at least one of --lastname/--firstname/--middlename is required".to_owned());
    }
    let Some(case) = case else {
        // Nominative: identity.
        return Ok([
            non_empty(&name.lastname),
            non_empty(&name.firstname),
            non_empty(&name.middlename),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" "));
    };
    Ok(decline_parts(name, gender, case).join(" "))
}

fn decline_appointment(
    appointment: &str,
    office: &str,
    case: Option<Case>,
) -> Result<String, String> {
    let appointment = appointment.trim();
    if appointment.is_empty() {
        return Err("--appointment is required".to_owned());
    }
    if office.trim().is_empty() {
        Ok(petrovich::decline_appointment(appointment, case))
    } else {
        Ok(petrovich::decline_full_appointment(
            appointment,
            office.trim(),
            case,
        ))
    }
}

fn run_batch(source: &str, gender: GenderArg, case: Option<Case>) -> Result<(), String> {
    let input: Box<dyn Read> = if source == "-" {
        Box::new(std::io::stdin())
    } else {
        Box::new(
            std::fs::File::open(source)
                .map_err(|e| format!("cannot open batch file {source:?}: {e}"))?,
        )
    };
    let mut out = String::new();
    for (line_no, line) in BufReader::new(input).lines().enumerate() {
        let line = line.map_err(|e| format!("cannot read line {}: {e}", line_no + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let name = NameArgs {
            lastname: fields.next().unwrap_or("").to_owned().into(),
            firstname: fields.next().unwrap_or("").to_owned().into(),
            middlename: fields.next().unwrap_or("").to_owned().into(),
        };
        let gender = gender.resolve(&name);
        out.push_str(
            &decline_line(&name, gender, case).map_err(|e| format!("line {}: {e}", line_no + 1))?,
        );
        out.push('\n');
    }
    print!("{out}");
    Ok(())
}

fn run_appoint_batch(source: &str, case: Option<Case>) -> Result<(), String> {
    let input: Box<dyn Read> = if source == "-" {
        Box::new(std::io::stdin())
    } else {
        Box::new(
            std::fs::File::open(source)
                .map_err(|e| format!("cannot open batch file {source:?}: {e}"))?,
        )
    };
    let mut out = String::new();
    for (line_no, line) in BufReader::new(input).lines().enumerate() {
        let line = line.map_err(|e| format!("cannot read line {}: {e}", line_no + 1))?;
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let appointment = fields.next().unwrap_or("");
        let office = fields.next().unwrap_or("");
        out.push_str(
            &decline_appointment(appointment, office, case)
                .map_err(|e| format!("line {}: {e}", line_no + 1))?,
        );
        out.push('\n');
    }
    print!("{out}");
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    let result = match &cli.command {
        Command::Decline(args) => {
            if let Some(batch) = &args.batch {
                run_batch(batch, args.gender, args.case.case())
            } else {
                let gender = args.gender.resolve(&args.name);
                decline_line(&args.name, gender, args.case.case()).map(|line| {
                    println!("{line}");
                })
            }
        }
        Command::Appoint(args) => {
            if let Some(batch) = &args.batch {
                run_appoint_batch(batch, args.case.case())
            } else {
                decline_appointment(
                    args.appointment.as_deref().unwrap_or(""),
                    args.office.as_deref().unwrap_or(""),
                    args.case.case(),
                )
                .map(|line| {
                    println!("{line}");
                })
            }
        }
        Command::Gender(name) => {
            if non_empty(&name.lastname).is_none()
                && non_empty(&name.firstname).is_none()
                && non_empty(&name.middlename).is_none()
            {
                Err("at least one of --lastname/--firstname/--middlename is required".to_owned())
            } else {
                let gender = detect_gender(
                    non_empty(&name.lastname),
                    non_empty(&name.firstname),
                    non_empty(&name.middlename),
                );
                println!(
                    "{}",
                    match gender {
                        Gender::Male => "male",
                        Gender::Female => "female",
                        Gender::Androgynous => "androgynous",
                    }
                );
                Ok(())
            }
        }
    };
    if let Err(message) = result {
        eprintln!("petrovich: error: {message}");
        std::process::exit(1);
    }
}
