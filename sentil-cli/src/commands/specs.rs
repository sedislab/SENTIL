//! `sentil specs` lists the premade specifications

use std::collections::BTreeMap;

use sentil::SpecRegistry;
use serde_json::json;

use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

pub fn run(name: Option<&str>, filter: Option<&str>, out: &Out) -> Run {
    let registry = SpecRegistry::global();
    match name {
        Some(name) => inspect(registry, name, out),
        None => list(registry, filter, out),
    }
}

fn list(registry: &SpecRegistry, filter: Option<&str>, out: &Out) -> Run {
    let mut names = registry.available();
    names.sort();
    if let Some(text) = filter {
        names.retain(|n| n.contains(text));
    }

    if out.is_text() {
        let mut by_domain: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for name in &names {
            let (domain, _) = name.split_once('/').unwrap_or(("other", name));
            by_domain.entry(domain).or_default().push(name);
        }
        for (domain, entries) in &by_domain {
            println!("{}", out.paint(domain, output::heading()));
            for entry in entries {
                println!("  {entry}");
            }
        }
        let footer = format!(
            "\n{} specifications. inspect one with `sentil specs <name>`.",
            names.len()
        );
        println!("{}", out.paint(&footer, output::dim()));
    } else {
        let specs: Vec<_> = names
            .iter()
            .map(|name| {
                let (domain, short) = name.split_once('/').unwrap_or(("other", name));
                json!({ "name": name, "domain": domain, "short": short })
            })
            .collect();
        println!(
            "{}",
            json!({ "schema_version": "1.0", "verb": "specs", "specs": specs })
        );
    }
    Ok(code::SUCCESS)
}

fn inspect(registry: &SpecRegistry, name: &str, out: &Out) -> Run {
    let builder = registry.builder(name).map_err(|e| {
        CliError::Input(
            format!("no specification named '{name}': {e}"),
            Some("run `sentil specs` to list the available specifications".into()),
        )
    })?;
    let template = builder.template();
    let resolved = builder.parameters();
    let variants = builder.available_variants();

    if out.is_text() {
        println!("{}", out.paint(&template.metadata.name, output::heading()));
        println!("  domain: {}", template.metadata.domain);
        if let Some(description) = &template.metadata.description {
            println!("  {description}");
        }

        println!("\n{}", out.paint("formula", output::heading()));
        println!("  deterministic: {}", template.formulas.deterministic);
        match &template.formulas.probabilistic {
            Some(prob) => println!("  probabilistic: {prob}"),
            None => println!("  {}", out.paint("probabilistic: none", output::dim())),
        }

        if !resolved.is_empty() {
            println!("\n{}", out.paint("parameters", output::heading()));
            let mut keys: Vec<_> = resolved.keys().collect();
            keys.sort();
            for key in keys {
                let value = resolved[key];
                let def = template.parameters.get(key);
                let unit = def
                    .and_then(|d| d.unit.as_deref())
                    .map(|u| format!(" {u}"))
                    .unwrap_or_default();
                let range = def
                    .and_then(|d| d.range)
                    .map(|[lo, hi]| format!("  range [{lo}, {hi}]"))
                    .unwrap_or_default();
                println!("  {key} = {value}{unit}{}", out.paint(&range, output::dim()));
            }
        }

        println!("\n{}", out.paint("variants", output::heading()));
        if variants.is_empty() {
            println!("  {}", out.paint("none", output::dim()));
        } else {
            for variant in &variants {
                println!("  {variant}");
            }
        }

        if let Some(references) = &template.metadata.references {
            if !references.is_empty() {
                println!("\n{}", out.paint("references", output::heading()));
                for reference in references {
                    println!("  {reference}");
                }
            }
        }
    } else {
        let parameters: BTreeMap<_, _> = resolved
            .iter()
            .map(|(key, value)| {
                let def = template.parameters.get(key);
                (
                    key.clone(),
                    json!({
                        "value": value,
                        "unit": def.and_then(|d| d.unit.clone()),
                        "range": def.and_then(|d| d.range),
                        "description": def.and_then(|d| d.description.clone()),
                    }),
                )
            })
            .collect();
        println!(
            "{}",
            json!({
                "schema_version": "1.0",
                "verb": "specs",
                "name": template.metadata.name,
                "domain": template.metadata.domain,
                "description": template.metadata.description,
                "formula": {
                    "deterministic": template.formulas.deterministic,
                    "probabilistic": template.formulas.probabilistic,
                },
                "parameters": parameters,
                "variants": variants,
                "references": template.metadata.references,
            })
        );
    }
    Ok(code::SUCCESS)
}