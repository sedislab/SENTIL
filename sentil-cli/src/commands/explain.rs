//! `sentil explain`

use crate::error::{code, CliError, Run};
use crate::output::{self, Out};

struct Operator {
    name: &'static str,
    grammar: &'static str,
    semantics: &'static str,
}

const OPERATORS: &[Operator] = &[
    Operator {
        name: "predicate",
        grammar: "f(x) ~ c, where ~ is one of > >= < <= == !=",
        semantics: "the signed margin f(x) - c (flipped for < and <=), so a positive value is how far the comparison holds and a negative one how far it fails",
    },
    Operator {
        name: "not",
        grammar: "not phi  (also !phi)",
        semantics: "negation flips the sign of the robustness",
    },
    Operator {
        name: "and",
        grammar: "phi and psi  (also phi & psi)",
        semantics: "conjunction takes the minimum of the two robustness values",
    },
    Operator {
        name: "or",
        grammar: "phi or psi  (also phi | psi)",
        semantics: "disjunction takes the maximum of the two robustness values",
    },
    Operator {
        name: "implies",
        grammar: "phi -> psi",
        semantics: "implication is max(-rho(phi), rho(psi))",
    },
    Operator {
        name: "always",
        grammar: "always[a, b] phi  (unbounded as always phi)",
        semantics: "the infimum of robustness over [t+a, t+b]; the property holds throughout the window",
    },
    Operator {
        name: "eventually",
        grammar: "eventually[a, b] phi",
        semantics: "the supremum of robustness over [t+a, t+b]; the property holds somewhere in the window",
    },
    Operator {
        name: "until",
        grammar: "phi until[a, b] psi",
        semantics: "sup over s in [t+a, t+b] of min(rho(psi, s), inf over r in [t, s] of rho(phi, r))",
    },
    Operator {
        name: "historically",
        grammar: "historically[a, b] phi",
        semantics: "the past dual of always: the infimum of robustness over [t-b, t-a], so it resolves from samples already seen",
    },
    Operator {
        name: "once",
        grammar: "once[a, b] phi",
        semantics: "the past dual of eventually: the supremum of robustness over [t-b, t-a]",
    },
    Operator {
        name: "since",
        grammar: "phi since[a, b] psi",
        semantics: "the past dual of until",
    },
    Operator {
        name: "next",
        grammar: "next phi",
        semantics: "shifts evaluation one step forward, returning negative infinity at the end of the trace",
    },
    Operator {
        name: "probabilistic",
        grammar: "P>=p(phi)  (also P>p, P<=p, P<p)",
        semantics: "plus or minus infinity according to whether the estimated satisfaction probability meets p; estimate the probability with `sentil smc`",
    },
];

struct Fields {
    verb: &'static str,
    lines: &'static [&'static str],
}

const FIELDS: &[Fields] = &[
    Fields {
        verb: "check",
        lines: &[
            "verb            always \"check\"",
            "formula         the formula that was evaluated",
            "trace           the trace path or -",
            "semantics       dense or discrete",
            "verdict         satisfied or violated",
            "robustness      the signed robustness value",
            "backend         cpu",
            "elapsed_ms      wall-clock evaluation time",
        ],
    },
    Fields {
        verb: "smc",
        lines: &[
            "verb            always \"smc\"",
            "algorithm       smc, sprt, or chernoff",
            "samples         the number of realizations drawn",
            "satisfactions   how many of them satisfied the formula",
            "probability     the point estimate",
            "interval        {method, confidence, low, high} for smc and chernoff",
            "elapsed_ms      wall-clock simulation time",
        ],
    },
    Fields {
        verb: "monitor",
        lines: &[
            "event           \"sample\" per input line, then one \"summary\"",
            "time            the sample timestamp (sample records)",
            "results         {id: {robustness, resolved}} (sample records)",
            "samples         the total sample count (summary record)",
        ],
    },
];

pub fn run(topic: Option<&str>, fields: bool, out: &Out) -> Run {
    match (topic, fields) {
        (Some(verb), true) => explain_fields(verb, out),
        (Some(name), false) => explain_operator(name, out),
        (None, _) => {
            list(out);
            Ok(code::SUCCESS)
        }
    }
}

fn explain_operator(name: &str, out: &Out) -> Run {
    let operator = OPERATORS.iter().find(|o| o.name == name).ok_or_else(|| {
        CliError::Input(
            format!("no operator named '{name}'"),
            Some("run `sentil explain` to list the operators".into()),
        )
    })?;
    println!("{}", out.paint(operator.name, output::heading()));
    println!("  grammar    {}", operator.grammar);
    println!("  robustness {}", operator.semantics);
    Ok(code::SUCCESS)
}

fn explain_fields(verb: &str, out: &Out) -> Run {
    let entry = FIELDS.iter().find(|f| f.verb == verb).ok_or_else(|| {
        CliError::Input(
            format!("no output fields documented for '{verb}'"),
            Some("try check, smc, or monitor".into()),
        )
    })?;
    println!(
        "{}",
        out.paint(&format!("{verb} output fields"), output::heading())
    );
    println!("  every record carries schema_version \"1.0\".");
    for line in entry.lines {
        println!("  {line}");
    }
    Ok(code::SUCCESS)
}

fn list(out: &Out) {
    println!("{}", out.paint("operators", output::heading()));
    for operator in OPERATORS {
        println!("  {}", operator.name);
    }
    println!(
        "\n{}",
        out.paint(
            "explain one with `sentil explain <operator>`, or a verb's JSON with `sentil explain --fields check`.",
            output::dim()
        )
    );
}