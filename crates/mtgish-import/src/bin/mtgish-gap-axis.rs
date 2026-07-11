//! Group unsupported mtgish cards by the leading-parenthesis gap axis.
//!
//! Usage:
//! ```text
//!   mtgish-gap-axis [<mtgish-cards.json>] [<report.json>]
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use mtgish_import::convert::convert_card;
use mtgish_import::report::{Ctx, ImportReport};
use mtgish_import::schema::types::OracleCard;
use serde::Serialize;
use serde_json::{json, Value};

struct Args {
    mtgish_path: PathBuf,
    report_path: Option<PathBuf>,
}

#[derive(Default)]
struct AxisStat {
    count: usize,
    cards: BTreeSet<String>,
    paths: BTreeMap<String, PathStat>,
}

#[derive(Default)]
struct PathStat {
    count: usize,
    cards: BTreeSet<String>,
}

#[derive(Serialize)]
struct SerializableAxis {
    axis: String,
    count: usize,
    cards: Vec<String>,
    paths: Vec<SerializablePath>,
}

#[derive(Serialize)]
struct SerializablePath {
    path: String,
    count: usize,
    cards: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => {
            let serialized = serde_json::to_string_pretty(&report).expect("report serializes");
            let report_path = parse_args().report_path;
            match report_path {
                Some(path) => {
                    if let Err(e) = write_report(&path, &serialized) {
                        eprintln!("mtgish-gap-axis: {e:#}");
                        return ExitCode::FAILURE;
                    }
                    println!("wrote gap axis report -> {}", path.display());
                }
                None => println!("{serialized}"),
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("mtgish-gap-axis: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<Value> {
    let args = parse_args();
    let raw_cards = fs::read_to_string(&args.mtgish_path)
        .with_context(|| format!("reading {}", args.mtgish_path.display()))?;
    let raw_values: Vec<Value> =
        serde_json::from_str(&raw_cards).context("parsing mtgish cards")?;

    let mut cards_total = 0usize;
    let mut clean_cards = 0usize;
    let mut unsupported_cards = 0usize;
    let mut deserialize_failures = Vec::new();
    let mut axes: BTreeMap<String, AxisStat> = BTreeMap::new();

    for raw in raw_values {
        cards_total += 1;
        let card_name = display_name_from_raw(&raw);
        let card: OracleCard = match serde_json::from_value(raw) {
            Ok(card) => card,
            Err(e) => {
                deserialize_failures.push(json!({
                    "card_name": card_name,
                    "error": e.to_string(),
                }));
                continue;
            }
        };

        let mut per_card = ImportReport::default();
        let mut ctx = Ctx::new(card_name.clone(), &mut per_card);
        let converted = convert_card(&card, &mut ctx).is_ok();
        let saw_gap = ctx.finish();
        if converted && !saw_gap {
            clean_cards += 1;
            continue;
        }
        if !saw_gap {
            continue;
        }
        unsupported_cards += 1;

        for (path, stat) in per_card.unsupported {
            let axis = leading_parenthesis_axis(&path);
            let axis_stat = axes.entry(axis).or_default();
            axis_stat.count += stat.count;
            axis_stat.cards.insert(card_name.clone());
            let path_stat = axis_stat.paths.entry(path).or_default();
            path_stat.count += stat.count;
            path_stat.cards.insert(card_name.clone());
        }
    }

    let mut ranked_axes = axes
        .into_iter()
        .map(|(axis, stat)| {
            let mut paths = stat
                .paths
                .into_iter()
                .map(|(path, path_stat)| SerializablePath {
                    path,
                    count: path_stat.count,
                    cards: path_stat.cards.into_iter().collect(),
                })
                .collect::<Vec<_>>();
            paths.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.path.cmp(&b.path)));
            SerializableAxis {
                axis,
                count: stat.count,
                cards: stat.cards.into_iter().collect(),
                paths,
            }
        })
        .collect::<Vec<_>>();
    ranked_axes.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.axis.cmp(&b.axis)));

    let best_axis = ranked_axes.first().map(|axis| {
        json!({
            "axis": axis.axis,
            "count": axis.count,
            "card_count": axis.cards.len(),
        })
    });

    Ok(json!({
        "summary": {
            "cards_total": cards_total,
            "clean_cards": clean_cards,
            "clean_percent": percent(clean_cards, cards_total),
            "unsupported_cards": unsupported_cards,
            "deserialize_failures": deserialize_failures.len(),
            "axis_count": ranked_axes.len(),
            "best_axis": best_axis,
        },
        "deserialize_failures": deserialize_failures,
        "axes": ranked_axes,
    }))
}

fn parse_args() -> Args {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    Args {
        mtgish_path: args
            .first()
            .cloned()
            .unwrap_or_else(|| "data/mtgish-cards.json".to_string())
            .into(),
        report_path: args.get(1).map(PathBuf::from),
    }
}

fn write_report(path: &Path, serialized: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }
    fs::write(path, serialized).with_context(|| format!("writing {}", path.display()))
}

fn display_name_from_raw(raw: &Value) -> String {
    let mut names = Vec::new();
    collect_raw_names(raw, &mut names);
    if names.is_empty() {
        "<no name>".to_string()
    } else {
        names.join(" // ")
    }
}

fn collect_raw_names(value: &Value, names: &mut Vec<String>) {
    if let Some(name) = value.get("Name").and_then(Value::as_str) {
        names.push(name.to_string());
        return;
    }

    for key in [
        "FrontFace",
        "BackFace",
        "Adventure",
        "Prepared",
        "Omen",
        "Unflipped",
        "Flipped",
        "LeftDoor",
        "RightDoor",
    ] {
        if let Some(child) = value.get(key) {
            collect_raw_names(child, names);
        }
    }

    if let Some(cards) = value.get("Cards").and_then(Value::as_array) {
        for child in cards {
            collect_raw_names(child, names);
        }
    }
}

fn leading_parenthesis_axis(path: &str) -> String {
    let detail = path.split_once(" :: ").map_or(path, |(_, detail)| detail);
    let without_json = detail.split_once(" :: {").map_or(detail, |(head, _)| head);
    let axis = without_json
        .split_once('(')
        .map_or(without_json, |(head, _)| head)
        .trim();
    if axis.is_empty() {
        "<empty>".to_string()
    } else {
        axis.to_string()
    }
}

fn percent(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        ((numerator as f64 / denominator as f64) * 10_000.0).round() / 100.0
    }
}
