//! A compact binary form of a parsed formula.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use sentil::formula::{BinaryOp, ComparisonOp, Expr, Formula, Interval, Predicate, ProbabilityOp};

const HEADER: [u8; 4] = *b"SEN1";

// Matches the parser's nesting cap.
const MAX_DEPTH: usize = 256;

/// The compiled blob was not a well-formed formula.
#[derive(Debug, PartialEq, Eq)]
pub struct Malformed;

/// Serialises a formula to its compact byte form.
pub fn encode(formula: &Formula) -> Vec<u8> {
    let mut w = Writer { bytes: Vec::new() };
    w.bytes.extend_from_slice(&HEADER);
    w.formula(formula);
    w.bytes
}

/// Rebuilds a formula from [`encode`] output.
pub fn decode(bytes: &[u8]) -> Result<Formula, Malformed> {
    let mut r = Reader { bytes, pos: 0 };
    if r.take(4)? != HEADER {
        return Err(Malformed);
    }
    let formula = r.formula(0)?;
    if r.pos != bytes.len() {
        return Err(Malformed);
    }
    Ok(formula)
}

struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    fn len(&mut self, n: usize) {
        self.bytes.extend_from_slice(&(n as u32).to_le_bytes());
    }

    fn f64(&mut self, v: f64) {
        self.bytes.extend_from_slice(&v.to_le_bytes());
    }

    fn string(&mut self, s: &str) {
        self.len(s.len());
        self.bytes.extend_from_slice(s.as_bytes());
    }

    fn interval(&mut self, iv: &Interval) {
        self.f64(iv.lower());
        match iv.upper() {
            Some(upper) => {
                self.bytes.push(1);
                self.f64(upper);
            }
            None => self.bytes.push(0),
        }
    }

    fn formula(&mut self, f: &Formula) {
        match f {
            Formula::Predicate(p) => {
                self.bytes.push(0);
                self.expr(&p.lhs);
                self.bytes.push(cmp_tag(p.op));
                self.expr(&p.rhs);
            }
            Formula::Not(a) => {
                self.bytes.push(1);
                self.formula(a);
            }
            Formula::And(a, b) => self.binary(2, a, b),
            Formula::Or(a, b) => self.binary(3, a, b),
            Formula::Implies(a, b) => self.binary(4, a, b),
            Formula::Always(iv, a) => self.temporal(5, iv, a),
            Formula::Eventually(iv, a) => self.temporal(6, iv, a),
            Formula::Until(iv, a, b) => {
                self.bytes.push(7);
                self.interval(iv);
                self.formula(a);
                self.formula(b);
            }
            Formula::Next(a) => {
                self.bytes.push(8);
                self.formula(a);
            }
            Formula::Since(iv, a, b) => {
                self.bytes.push(9);
                self.interval(iv);
                self.formula(a);
                self.formula(b);
            }
            Formula::Historically(iv, a) => self.temporal(10, iv, a),
            Formula::Once(iv, a) => self.temporal(11, iv, a),
            Formula::Probabilistic(op, p, a) => {
                self.bytes.push(12);
                self.bytes.push(prob_tag(*op));
                self.f64(*p);
                self.formula(a);
            }
        }
    }

    fn binary(&mut self, tag: u8, a: &Formula, b: &Formula) {
        self.bytes.push(tag);
        self.formula(a);
        self.formula(b);
    }

    fn temporal(&mut self, tag: u8, iv: &Interval, a: &Formula) {
        self.bytes.push(tag);
        self.interval(iv);
        self.formula(a);
    }

    fn expr(&mut self, e: &Expr) {
        match e {
            Expr::Binary(op, a, b) => {
                self.bytes.push(0);
                self.bytes.push(bin_tag(*op));
                self.expr(a);
                self.expr(b);
            }
            Expr::Call(name, args) => {
                self.bytes.push(1);
                self.string(name);
                self.len(args.len());
                for arg in args {
                    self.expr(arg);
                }
            }
            Expr::Literal(v) => {
                self.bytes.push(2);
                self.f64(*v);
            }
            Expr::Variable(name) => {
                self.bytes.push(3);
                self.string(name);
            }
        }
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        let end = self.pos.checked_add(n).ok_or(Malformed)?;
        let slice = self.bytes.get(self.pos..end).ok_or(Malformed)?;
        self.pos = end;
        Ok(slice)
    }

    fn byte(&mut self) -> Result<u8, Malformed> {
        Ok(self.take(1)?[0])
    }

    fn len(&mut self) -> Result<usize, Malformed> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
    }

    fn f64(&mut self) -> Result<f64, Malformed> {
        let b = self.take(8)?;
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(b);
        Ok(f64::from_le_bytes(bytes))
    }

    fn string(&mut self) -> Result<String, Malformed> {
        let len = self.len()?;
        let bytes = self.take(len)?;
        core::str::from_utf8(bytes)
            .map(String::from)
            .map_err(|_| Malformed)
    }

    fn interval(&mut self) -> Result<Interval, Malformed> {
        let lower = self.f64()?;
        let upper = match self.byte()? {
            0 => None,
            1 => Some(self.f64()?),
            _ => return Err(Malformed),
        };
        Interval::new(lower, upper).map_err(|_| Malformed)
    }

    fn formula(&mut self, depth: usize) -> Result<Formula, Malformed> {
        if depth > MAX_DEPTH {
            return Err(Malformed);
        }
        let deeper = depth + 1;
        match self.byte()? {
            0 => {
                let lhs = self.expr(deeper)?;
                let op = decode_cmp(self.byte()?)?;
                let rhs = self.expr(deeper)?;
                Ok(Formula::Predicate(Predicate { lhs, op, rhs }))
            }
            1 => Ok(Formula::Not(self.boxed(deeper)?)),
            2 => Ok(Formula::And(self.boxed(deeper)?, self.boxed(deeper)?)),
            3 => Ok(Formula::Or(self.boxed(deeper)?, self.boxed(deeper)?)),
            4 => Ok(Formula::Implies(self.boxed(deeper)?, self.boxed(deeper)?)),
            5 => Ok(Formula::Always(self.interval()?, self.boxed(deeper)?)),
            6 => Ok(Formula::Eventually(self.interval()?, self.boxed(deeper)?)),
            7 => {
                let iv = self.interval()?;
                Ok(Formula::Until(iv, self.boxed(deeper)?, self.boxed(deeper)?))
            }
            8 => Ok(Formula::Next(self.boxed(deeper)?)),
            9 => {
                let iv = self.interval()?;
                Ok(Formula::Since(iv, self.boxed(deeper)?, self.boxed(deeper)?))
            }
            10 => Ok(Formula::Historically(self.interval()?, self.boxed(deeper)?)),
            11 => Ok(Formula::Once(self.interval()?, self.boxed(deeper)?)),
            12 => {
                let op = decode_prob(self.byte()?)?;
                let threshold = self.f64()?;
                Ok(Formula::Probabilistic(op, threshold, self.boxed(deeper)?))
            }
            _ => Err(Malformed),
        }
    }

    fn boxed(&mut self, depth: usize) -> Result<Box<Formula>, Malformed> {
        Ok(Box::new(self.formula(depth)?))
    }

    fn expr(&mut self, depth: usize) -> Result<Expr, Malformed> {
        if depth > MAX_DEPTH {
            return Err(Malformed);
        }
        let deeper = depth + 1;
        match self.byte()? {
            0 => {
                let op = decode_bin(self.byte()?)?;
                let lhs = Box::new(self.expr(deeper)?);
                let rhs = Box::new(self.expr(deeper)?);
                Ok(Expr::Binary(op, lhs, rhs))
            }
            1 => {
                let name = self.string()?;
                let count = self.len()?;
                let mut args = Vec::with_capacity(count.min(64));
                for _ in 0..count {
                    args.push(self.expr(deeper)?);
                }
                Ok(Expr::Call(name, args))
            }
            2 => Ok(Expr::Literal(self.f64()?)),
            3 => Ok(Expr::Variable(self.string()?)),
            _ => Err(Malformed),
        }
    }
}

fn cmp_tag(op: ComparisonOp) -> u8 {
    match op {
        ComparisonOp::Less => 0,
        ComparisonOp::LessEqual => 1,
        ComparisonOp::Greater => 2,
        ComparisonOp::GreaterEqual => 3,
        ComparisonOp::Equal => 4,
        ComparisonOp::NotEqual => 5,
    }
}

fn decode_cmp(tag: u8) -> Result<ComparisonOp, Malformed> {
    match tag {
        0 => Ok(ComparisonOp::Less),
        1 => Ok(ComparisonOp::LessEqual),
        2 => Ok(ComparisonOp::Greater),
        3 => Ok(ComparisonOp::GreaterEqual),
        4 => Ok(ComparisonOp::Equal),
        5 => Ok(ComparisonOp::NotEqual),
        _ => Err(Malformed),
    }
}

fn bin_tag(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Add => 0,
        BinaryOp::Sub => 1,
        BinaryOp::Mul => 2,
        BinaryOp::Div => 3,
        BinaryOp::Mod => 4,
        BinaryOp::Pow => 5,
    }
}

fn decode_bin(tag: u8) -> Result<BinaryOp, Malformed> {
    match tag {
        0 => Ok(BinaryOp::Add),
        1 => Ok(BinaryOp::Sub),
        2 => Ok(BinaryOp::Mul),
        3 => Ok(BinaryOp::Div),
        4 => Ok(BinaryOp::Mod),
        5 => Ok(BinaryOp::Pow),
        _ => Err(Malformed),
    }
}

fn prob_tag(op: ProbabilityOp) -> u8 {
    match op {
        ProbabilityOp::GreaterEqual => 0,
        ProbabilityOp::Greater => 1,
        ProbabilityOp::LessEqual => 2,
        ProbabilityOp::Less => 3,
    }
}

fn decode_prob(tag: u8) -> Result<ProbabilityOp, Malformed> {
    match tag {
        0 => Ok(ProbabilityOp::GreaterEqual),
        1 => Ok(ProbabilityOp::Greater),
        2 => Ok(ProbabilityOp::LessEqual),
        3 => Ok(ProbabilityOp::Less),
        _ => Err(Malformed),
    }
}

#[cfg(all(test, feature = "parser"))]
mod tests {
    use super::*;

    fn round_trip(text: &str) {
        let original = Formula::parse(text).expect("parse");
        let bytes = encode(&original);
        let restored = decode(&bytes).expect("decode");
        assert_eq!(original, restored, "round trip changed `{text}`");
    }

    #[test]
    fn round_trips_every_operator() {
        for text in [
            "x > 0",
            "x + 1 <= 2 * y",
            "abs(x - y) < 0.5",
            "not (a > 1) and (b < 2 or c >= 3)",
            "always[0, 10](speed < 5)",
            "eventually[1, 2](x > 0) until[0, 5](y < 1)",
            "next (x > 0)",
            "historically[0, 3](x > 0) since[0, 1](y > 0)",
            "once[0, 2](x > 0)",
            "P>=0.9(always[0, 5](x > 0))",
        ] {
            round_trip(text);
        }
    }

    #[test]
    fn rejects_truncated_and_trailing_and_bad_tag() {
        let bytes = encode(&Formula::parse("x > 0").unwrap());
        assert!(decode(&bytes[..bytes.len() - 1]).is_err());
        let mut trailing = bytes.clone();
        trailing.push(0);
        assert!(decode(&trailing).is_err());
        assert!(decode(b"SEN1\x7f").is_err());
        assert!(decode(b"XXXX").is_err());
        assert!(decode(&[]).is_err());
    }
}