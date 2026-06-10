//! ADR 0085 vault query expression parser: brace groups (OR), predicates inside groups (AND).

use serde_yaml::{Mapping, Value};

/// Top-level expression: one or more brace groups, OR’d together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryExpr {
    pub(crate) groups: Vec<BraceGroup>,
}

/// One `{ ... }` group: predicates are AND’d.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BraceGroup {
    pub(crate) predicates: Vec<Predicate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Predicate {
    /// `field='value'`
    Eq { field: String, value: String },
    /// `field=['a','b']` — any listed value may match (OR within the list).
    InList { field: String, values: Vec<String> },
    /// `field={'a','b'}` — every listed value must be present (AND within the list).
    AllInList { field: String, values: Vec<String> },
    /// `field@='substring'`
    Substring { field: String, needle: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub(crate) offset: usize,
    pub(crate) message: String,
}

impl ParseError {
    fn new(offset: usize, message: impl Into<String>) -> Self {
        Self { offset, message: message.into() }
    }
}

/// Parse a full ADR 0085 expression. Whitespace is allowed between tokens and groups.
///
/// # Errors
///
/// Returns [`ParseError`] with a byte offset into the original trimmed input when the expression
/// is not well-formed.
pub fn parse_query_expression(input: &str) -> Result<QueryExpr, ParseError> {
    let input = input.trim();
    if input.is_empty() {
        return Err(ParseError::new(
            0,
            "expected at least one `{...}` group (got empty expression)",
        ));
    }

    let mut p = Parser::new(input, 0);
    let mut groups = Vec::new();
    p.skip_ws();
    while !p.is_eof() {
        p.expect_char('{')?;
        let inner_start = p.pos;
        let close_idx = find_closing_brace(input, inner_start)?;
        let inner = &input[inner_start..close_idx];
        p.pos = close_idx + 1;
        groups.push(parse_brace_group(inner, inner_start)?);
        p.skip_ws();
    }

    Ok(QueryExpr { groups })
}

fn parse_brace_group(inner: &str, base_offset: usize) -> Result<BraceGroup, ParseError> {
    let mut p = Parser::new(inner, base_offset);
    p.skip_ws();
    let mut predicates = Vec::new();
    while !p.is_eof() {
        predicates.push(parse_predicate(&mut p)?);
        p.skip_ws();
    }

    if predicates.is_empty() {
        return Err(ParseError::new(
            base_offset,
            "empty brace group `{}`: need at least one predicate",
        ));
    }

    Ok(BraceGroup { predicates })
}

fn parse_predicate(p: &mut Parser<'_>) -> Result<Predicate, ParseError> {
    let field = parse_ident(p)?;
    p.skip_ws();
    if p.try_consume_str("@=") {
        let needle = parse_quoted(p)?;
        return Ok(Predicate::Substring { field, needle });
    }
    p.expect_char('=')?;
    p.skip_ws();
    match p.peek() {
        Some('\'') => {
            let value = parse_quoted(p)?;
            Ok(Predicate::Eq { field, value })
        }
        Some('[') => {
            let values = parse_list(p)?;
            Ok(Predicate::InList { field, values })
        }
        Some('{') => {
            let values = parse_braced_list(p)?;
            Ok(Predicate::AllInList { field, values })
        }
        Some(c) => Err(p.err(format!("expected `'` or `[` or `{{` after `=`, found {c:?}"))),
        None => Err(p.err("expected `'` or `[` or `{{` after `=`")),
    }
}

fn parse_ident(p: &mut Parser<'_>) -> Result<String, ParseError> {
    let start = p.pos;
    let first = p.peek().ok_or_else(|| p.err("expected field name"))?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(p.err("field name must start with a letter or `_`"));
    }
    p.pos += first.len_utf8();
    while let Some(c) = p.peek() {
        if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
            p.pos += c.len_utf8();
        } else {
            break;
        }
    }
    Ok(p.s[start..p.pos].to_string())
}

fn parse_list(p: &mut Parser<'_>) -> Result<Vec<String>, ParseError> {
    p.expect_char('[')?;
    p.skip_ws();
    let mut out = Vec::new();
    loop {
        p.skip_ws();
        if p.peek() == Some(']') {
            p.pos += 1;
            break;
        }
        out.push(parse_quoted(p)?);
        p.skip_ws();
        if p.peek() == Some(']') {
            p.pos += 1;
            break;
        }
        p.expect_char(',')?;
    }
    Ok(out)
}

fn parse_braced_list(p: &mut Parser<'_>) -> Result<Vec<String>, ParseError> {
    p.expect_char('{')?;
    p.skip_ws();
    let mut out = Vec::new();
    loop {
        p.skip_ws();
        if p.peek() == Some('}') {
            p.pos += 1;
            break;
        }
        out.push(parse_quoted(p)?);
        p.skip_ws();
        if p.peek() == Some('}') {
            p.pos += 1;
            break;
        }
        p.expect_char(',')?;
    }
    Ok(out)
}

fn parse_quoted(p: &mut Parser<'_>) -> Result<String, ParseError> {
    p.expect_char('\'')?;
    let mut out = String::new();
    loop {
        let c = match p.peek() {
            Some(c) => c,
            None => return Err(p.err("unclosed single-quoted string")),
        };
        if c == '\\' {
            p.pos += 1;
            let esc = p.peek().ok_or_else(|| p.err("unclosed string (escape at end)"))?;
            match esc {
                '\'' => out.push('\''),
                '\\' => out.push('\\'),
                _ => {
                    return Err(p.err("invalid escape (only `\\'` and `\\\\` allowed)"));
                }
            }
            p.pos += esc.len_utf8();
            continue;
        }
        if c == '\'' {
            p.pos += 1;
            return Ok(out);
        }
        p.pos += c.len_utf8();
        out.push(c);
    }
}

/// Find index of `}` that closes the `{` whose content begins at `start` (first byte inside `{`),
/// respecting nested `{...}` and single-quoted strings (so `tag={'a','b'}` does not end the outer group early).
fn find_closing_brace(s: &str, start: usize) -> Result<usize, ParseError> {
    let bytes = s.as_bytes();
    let mut i = start;
    let mut depth = 1usize;
    let mut in_quote = false;
    while i < bytes.len() {
        let b = bytes[i];
        if in_quote {
            if b == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if b == b'\'' {
                in_quote = false;
            }
            i += 1;
            continue;
        }
        if b == b'\'' {
            in_quote = true;
            i += 1;
            continue;
        }
        if b == b'{' {
            depth += 1;
            i += 1;
            continue;
        }
        if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return Ok(i);
            }
            i += 1;
            continue;
        }
        i += 1;
    }
    Err(ParseError::new(start.saturating_sub(1), "unclosed `{`: no matching `}`"))
}

struct Parser<'a> {
    s: &'a str,
    pos: usize,
    /// Byte offset in the original full expression for error reporting.
    base: usize,
}

impl<'a> Parser<'a> {
    const fn new(s: &'a str, base: usize) -> Self {
        Self { s, pos: 0, base }
    }

    fn err(&self, message: impl Into<String>) -> ParseError {
        ParseError::new(self.pos + self.base, message)
    }

    const fn is_eof(&self) -> bool {
        self.pos >= self.s.len()
    }

    fn peek(&self) -> Option<char> {
        self.s[self.pos..].chars().next()
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        match self.peek() {
            Some(c) if c == expected => {
                self.pos += c.len_utf8();
                Ok(())
            }
            Some(c) => Err(self.err(format!("expected {expected:?}, found {c:?}"))),
            None => Err(self.err(format!("expected {expected:?}, found end of input"))),
        }
    }

    fn try_consume_str(&mut self, pat: &str) -> bool {
        if self.s[self.pos..].starts_with(pat) {
            self.pos += pat.len();
            true
        } else {
            false
        }
    }
}

/// True if the document's frontmatter satisfies the expression (OR of brace groups, AND of predicates within a group).
#[must_use]
pub fn eval_query_expr(expr: &QueryExpr, mapping: &Mapping) -> bool {
    expr.groups
        .iter()
        .any(|group| group.predicates.iter().all(|pred| eval_predicate(pred, mapping)))
}

fn eval_predicate(pred: &Predicate, mapping: &Mapping) -> bool {
    match pred {
        Predicate::Eq { field, value } => {
            mapping_get(mapping, field).is_some_and(|v| yaml_value_eq_str(v, value))
        }
        Predicate::InList { field, values } => in_list_matches(mapping_get(mapping, field), values),
        Predicate::AllInList { field, values } => {
            all_in_list_matches(mapping_get(mapping, field), values)
        }
        Predicate::Substring { field, needle } => mapping_get(mapping, field)
            .is_some_and(|v| value_substring_haystack(v).contains(needle.as_str())),
    }
}

fn mapping_get<'a>(mapping: &'a Mapping, key: &str) -> Option<&'a Value> {
    mapping.get(Value::String(key.to_string()))
}

fn yaml_value_eq_str(value: &Value, expected: &str) -> bool {
    match value {
        Value::String(s) => s == expected,
        Value::Bool(b) => (if *b { "true" } else { "false" }) == expected,
        Value::Number(n) => n.to_string() == expected,
        Value::Null => expected == "null",
        Value::Sequence(seq) => seq.iter().any(|v| yaml_value_eq_str(v, expected)),
        _ => false,
    }
}

fn in_list_matches(value: Option<&Value>, needles: &[String]) -> bool {
    let Some(value) = value else {
        return false;
    };
    let mut acc = Vec::new();
    collect_scalar_strings(value, &mut acc);
    needles.iter().any(|n| acc.iter().any(|a| a == n))
}

/// Every `needles` value must appear among normalized scalars from the field. Empty `needles` is vacuously true
/// when the field is present; missing field never matches.
fn all_in_list_matches(value: Option<&Value>, needles: &[String]) -> bool {
    let Some(value) = value else {
        return false;
    };
    if needles.is_empty() {
        return true;
    }
    let mut acc = Vec::new();
    collect_scalar_strings(value, &mut acc);
    needles.iter().all(|n| acc.iter().any(|a| a == n))
}

fn collect_scalar_strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Bool(b) => out.push(if *b { "true".into() } else { "false".into() }),
        Value::Number(n) => out.push(n.to_string()),
        Value::Sequence(seq) => {
            for v in seq {
                collect_scalar_strings(v, out);
            }
        }
        _ => {}
    }
}

fn value_substring_haystack(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Sequence(seq) => {
            let mut parts = Vec::new();
            for v in seq {
                match v {
                    Value::String(s) => parts.push(s.clone()),
                    Value::Number(n) => parts.push(n.to_string()),
                    Value::Bool(b) => parts.push(if *b { "true".into() } else { "false".into() }),
                    _ => {}
                }
            }
            parts.join(" ")
        }
        _ => String::new(),
    }
}

#[cfg(test)]
#[path = "vault_query_test.rs"]
mod tests;
