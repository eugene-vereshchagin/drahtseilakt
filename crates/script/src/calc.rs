// The Calculator block's expression language: numbers, the variables a to z (the block's input parameters after the
// expression, a = the first), + - * / ^, parentheses, unary + and -, the constant pi and the functions below. Names
// are case-insensitive. Evaluated in double precision with every result rounded to single precision, as the C
// reference does (src/bb, Calculator: it knows numbers, variables, + - * /, parentheses and PI only; the rest is
// what the maps use: sin, cos, sqrt, ln, pi and ^ in Landfill in the Clouds). Hostile input is bounded: at most
// MAX_TOKENS tokens, nesting at most MAX_DEPTH, the expression at most MAX_EXPRESSION bytes (the C's buffer).

/// Most tokens an expression may have.
pub const MAX_TOKENS: usize = 256;
/// Deepest nesting of parentheses, function calls and unary operators.
pub const MAX_DEPTH: usize = 32;
/// The C reference reads at most this many bytes of the expression.
pub const MAX_EXPRESSION: usize = 511;

/// The functions of one argument.
pub const FUNCTIONS: &[&str] = &[
    "abs", "sqrt", "sin", "cos", "tan", "asin", "acos", "atan", "sinh", "cosh", "tanh", "exp", "ln", "log",
    "floor", "ceil",
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok<'a> {
    Num(f64),
    Var(usize),
    Name(&'a str),
    Op(u8),
    Open,
    Close,
}

fn lex(src: &[u8]) -> Option<Vec<Tok<'_>>> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while let Some(&c) = src.get(i) {
        if c == b' ' || c == b'\t' {
            i = i.saturating_add(1);
            continue;
        }
        if out.len() >= MAX_TOKENS {
            return None;
        }
        if c.is_ascii_digit() || c == b'.' {
            let start = i;
            while src.get(i).is_some_and(|d| d.is_ascii_digit()) {
                i = i.saturating_add(1);
            }
            if src.get(i) == Some(&b'.') {
                i = i.saturating_add(1);
                while src.get(i).is_some_and(|d| d.is_ascii_digit()) {
                    i = i.saturating_add(1);
                }
            }
            if matches!(src.get(i), Some(b'e' | b'E')) {
                let mut j = i.saturating_add(1);
                if matches!(src.get(j), Some(b'+' | b'-')) {
                    j = j.saturating_add(1);
                }
                if src.get(j).is_some_and(|d| d.is_ascii_digit()) {
                    while src.get(j).is_some_and(|d| d.is_ascii_digit()) {
                        j = j.saturating_add(1);
                    }
                    i = j;
                }
            }
            let text = std::str::from_utf8(src.get(start..i)?).ok()?;
            out.push(Tok::Num(text.parse::<f64>().ok()?));
        } else if c.is_ascii_alphabetic() {
            let start = i;
            while src.get(i).is_some_and(u8::is_ascii_alphabetic) {
                i = i.saturating_add(1);
            }
            let word = std::str::from_utf8(src.get(start..i)?).ok()?;
            if word.len() == 1 {
                let l = word.as_bytes().first()?.to_ascii_lowercase();
                out.push(Tok::Var(usize::from(l.saturating_sub(b'a'))));
            } else {
                out.push(Tok::Name(word));
            }
        } else if matches!(c, b'+' | b'-' | b'*' | b'/' | b'^') {
            out.push(Tok::Op(c));
            i = i.saturating_add(1);
        } else if c == b'(' {
            out.push(Tok::Open);
            i = i.saturating_add(1);
        } else if c == b')' {
            out.push(Tok::Close);
            i = i.saturating_add(1);
        } else {
            return None;
        }
    }
    Some(out)
}

fn round(x: f64) -> f64 {
    f64::from(x as f32)
}

struct Parser<'a, 'v> {
    toks: &'a [Tok<'a>],
    at: usize,
    vars: &'v dyn Fn(usize) -> Option<f32>,
}

impl<'a> Parser<'a, '_> {
    fn peek(&self) -> Option<Tok<'a>> {
        self.toks.get(self.at).copied()
    }

    fn expr(&mut self, depth: usize) -> Option<f64> {
        if depth > MAX_DEPTH {
            return None;
        }
        let mut v = self.term(depth)?;
        while let Some(Tok::Op(o @ (b'+' | b'-'))) = self.peek() {
            self.at = self.at.saturating_add(1);
            let r = self.term(depth)?;
            v = round(if o == b'+' { v + r } else { v - r });
        }
        Some(v)
    }

    fn term(&mut self, depth: usize) -> Option<f64> {
        let mut v = self.unary(depth)?;
        while let Some(Tok::Op(o @ (b'*' | b'/'))) = self.peek() {
            self.at = self.at.saturating_add(1);
            let r = self.unary(depth)?;
            v = round(if o == b'*' { v * r } else { v / r });
        }
        Some(v)
    }

    /// Unary signs bind looser than ^ (`-a^2` is `-(a^2)`), tighter than * and /.
    fn unary(&mut self, depth: usize) -> Option<f64> {
        if depth > MAX_DEPTH {
            return None;
        }
        match self.peek() {
            Some(Tok::Op(b'-')) => {
                self.at = self.at.saturating_add(1);
                Some(-self.unary(depth.saturating_add(1))?)
            }
            Some(Tok::Op(b'+')) => {
                self.at = self.at.saturating_add(1);
                self.unary(depth.saturating_add(1))
            }
            _ => self.power(depth),
        }
    }

    /// `^` is right associative.
    fn power(&mut self, depth: usize) -> Option<f64> {
        let base = self.primary(depth)?;
        if let Some(Tok::Op(b'^')) = self.peek() {
            self.at = self.at.saturating_add(1);
            let e = self.unary(depth.saturating_add(1))?;
            return Some(round(base.powf(e)));
        }
        Some(base)
    }

    fn primary(&mut self, depth: usize) -> Option<f64> {
        if depth > MAX_DEPTH {
            return None;
        }
        let t = self.peek()?;
        self.at = self.at.saturating_add(1);
        match t {
            Tok::Num(n) => Some(round(n)),
            Tok::Var(i) => (self.vars)(i).map(f64::from),
            Tok::Open => {
                let v = self.expr(depth.saturating_add(1))?;
                match self.peek() {
                    Some(Tok::Close) => {
                        self.at = self.at.saturating_add(1);
                        Some(v)
                    }
                    _ => None,
                }
            }
            Tok::Name(n) if n.eq_ignore_ascii_case("pi") => Some(f64::from(3.141_592_7_f32)),
            Tok::Name(n) => {
                if !matches!(self.peek(), Some(Tok::Open)) {
                    return None;
                }
                self.at = self.at.saturating_add(1);
                let a = self.expr(depth.saturating_add(1))?;
                if !matches!(self.peek(), Some(Tok::Close)) {
                    return None;
                }
                self.at = self.at.saturating_add(1);
                let r = match n.to_ascii_lowercase().as_str() {
                    "abs" => a.abs(),
                    "sqrt" => a.sqrt(),
                    "sin" => a.sin(),
                    "cos" => a.cos(),
                    "tan" => a.tan(),
                    "asin" => a.asin(),
                    "acos" => a.acos(),
                    "atan" => a.atan(),
                    "sinh" => a.sinh(),
                    "cosh" => a.cosh(),
                    "tanh" => a.tanh(),
                    "exp" => a.exp(),
                    "ln" => a.ln(),
                    "log" => a.log10(),
                    "floor" => a.floor(),
                    "ceil" => a.ceil(),
                    _ => return None,
                };
                Some(round(r))
            }
            _ => None,
        }
    }
}

/// The value of an expression, `vars(i)` giving variable i (a = 0), or `None` for a syntax error (or an unknown
/// variable, or an expression beyond the caps).
pub fn evaluate(expression: &[u8], vars: &dyn Fn(usize) -> Option<f32>) -> Option<f32> {
    let src = expression.get(..MAX_EXPRESSION).unwrap_or(expression);
    let toks = lex(src)?;
    if toks.is_empty() {
        return None;
    }
    let mut p = Parser {
        toks: &toks,
        at: 0,
        vars,
    };
    let v = p.expr(0)?;
    (p.at == toks.len()).then_some(v as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str, v: &[f32]) -> Option<f32> {
        evaluate(s.as_bytes(), &|i| v.get(i).copied())
    }

    #[test]
    fn hand_computed() {
        assert_eq!(ev("1+2*3", &[]), Some(7.0));
        assert_eq!(ev("(1+2)*3", &[]), Some(9.0));
        assert_eq!(ev("2^3^2", &[]), Some(512.0));
        assert_eq!(ev("-2^2", &[]), Some(-4.0));
        assert_eq!(ev("a*b+0.5", &[4.0, 3.0]), Some(12.5));
        assert_eq!(ev("(A^2+A)*2", &[3.0]), Some(24.0));
        assert_eq!(ev("SQRT(16)/2", &[]), Some(2.0));
        assert_eq!(ev("sin(pi/2)", &[]), Some(1.0));
        assert_eq!(ev("ln(1)", &[]), Some(0.0));
        assert_eq!(ev("2*-3", &[]), Some(-6.0));
        assert_eq!(ev("1/0", &[]), Some(f32::INFINITY));
        assert_eq!(ev("1e2+.5", &[]), Some(100.5));
    }

    #[test]
    fn syntax_errors_and_caps() {
        for bad in ["", "1+", "(1", "1)", "1 2", "a", "foo(1)", "sin 1", "1+*2", "#", "sin()", "pi(1)"] {
            assert_eq!(ev(bad, &[]), None, "{bad}");
        }
        // nesting beyond the cap
        let deep = format!("{}1{}", "(".repeat(40), ")".repeat(40));
        assert_eq!(ev(&deep, &[]), None);
        let ok = format!("{}1{}", "(".repeat(20), ")".repeat(20));
        assert_eq!(ev(&ok, &[]), Some(1.0));
        assert_eq!(ev(&"-".repeat(100), &[]), None);
        // tokens beyond the cap: 129 numbers and 128 plus signs is 257 tokens
        let long = vec!["1"; 129].join("+");
        assert_eq!(ev(&long, &[]), None);
        let fits = vec!["1"; 128].join("+");
        assert_eq!(ev(&fits, &[]), Some(128.0));
        // hostile bytes never panic
        assert_eq!(evaluate(&[0xff, 0xfe, b'1'], &|_| None), None);
        assert_eq!(
            evaluate(&vec![b'9'; 5000], &|_| None),
            evaluate(&vec![b'9'; 511], &|_| None)
        );
    }
}
