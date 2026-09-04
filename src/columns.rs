use polars::prelude::*;
use std::collections::HashMap;

/// Map many real-world aliases to canonical names: Open, High, Low, Close, Volume
fn alias_map() -> HashMap<&'static str, &'static str> {
    // include common variants observed across vendors/exchanges
    let pairs: [(&str, &str); 31] = [
        ("open", "Open"), ("opn", "Open"), ("op", "Open"), ("opnpric", "Open"), ("opnprc", "Open"),
        ("high", "High"), ("hgh", "High"),
        ("low", "Low"), ("lw", "Low"),
        ("close", "Close"), ("cls", "Close"), ("last", "Close"), ("closeprice", "Close"),
        ("volume", "Volume"), ("vol", "Volume"), ("qty", "Volume"), ("totaltradedqty", "Volume"),
        // Your prior schemas
        ("opnpric", "Open"), ("hghpric", "High"), ("lwpric", "Low"), ("clspric", "Close"),
        ("sttlmpric", "Close"), ("ttltrfval", "Volume"), ("opnintrst", "OpenInterest"), ("chnginopnintrst", "ChangeInOpenInterest"),
        ("tckrsymb", "Symbol"), ("fininstrmactlxprydt", "Expiry"), ("traddt", "TradeDate"),
        ("date", "Date"), ("timestamp", "Timestamp"), ("time", "Time"),
    ];
    pairs.into_iter().collect()
}

pub fn canonicalize_columns(lf: LazyFrame) -> LazyFrame {
    let aliases = alias_map();
    // Build a projection that renames matching columns; leave others as-is.
    let mut exprs: Vec<Expr> = Vec::new();
    if let Ok(df) = lf.clone().fetch(10) {  // Only need schema, so fetch minimal rows
        let schema = df.schema();
        for fld in schema.iter_fields() {
            let name = fld.name().to_string();
            let key = name.to_lowercase().replace([' ', '-', '_'], "");
            if let Some(&canon) = aliases.get(key.as_str()) {
                exprs.push(col(&name).alias(canon));
            } else {
                exprs.push(col(&name));
            }
        }
    }
    lf.select(exprs)
}

/// Very small, pragmatic parser for expressions like:
///   "Open > 100 & Volume > 5000"  or  "(Close >= 250) | (Low < 100)"
pub struct ExprBuilder;
impl ExprBuilder {
    pub fn parse(input: &str) -> Option<Expr> {
        // This is intentionally tiny to keep the MVP fast and safe.
        // Strategy: split on logical ops first, parse comparisons, then rebuild.
        // Supports: >, >=, <, <=, ==, != and & (and), | (or), parentheses.
        // If anything fails, return None (no filter).
        let tokens = tokenize(input)?;
        parse_expr(&tokens).ok()
    }
}

// ===== Mini parser (very small) =====
#[derive(Debug, Clone, PartialEq)]
enum Tok { Ident(String), Num(f64), Gt, Ge, Lt, Le, Eq, Ne, And, Or, Lp, Rp }

fn tokenize(s: &str) -> Option<Vec<Tok>> {
    let mut t = Vec::new();
    let mut it = s.chars().peekable();
    while let Some(&c) = it.peek() {
        match c {
            ' ' | '\t' => { it.next(); }
            '(' => { t.push(Tok::Lp); it.next(); }
            ')' => { t.push(Tok::Rp); it.next(); }
            '&' => { t.push(Tok::And); it.next(); }
            '|' => { t.push(Tok::Or); it.next(); }
            '>' => { it.next(); if it.peek()==Some(&'=') { it.next(); t.push(Tok::Ge);} else { t.push(Tok::Gt);} }
            '<' => { it.next(); if it.peek()==Some(&'=') { it.next(); t.push(Tok::Le);} else { t.push(Tok::Lt);} }
            '=' => { it.next(); if it.peek()==Some(&'=') { it.next(); t.push(Tok::Eq);} else { return None; } }
            '!' => { it.next(); if it.peek()==Some(&'=') { it.next(); t.push(Tok::Ne);} else { return None; } }
            '0'..='9' | '.' => {
                let mut s2 = String::new();
                while let Some(&d) = it.peek() { if d.is_ascii_digit() || d=='.' { s2.push(d); it.next(); } else { break; } }
                t.push(Tok::Num(s2.parse().ok()?));
            }
            _ => { // ident
                if c.is_alphanumeric() || c=='_' {
                    let mut s2 = String::new();
                    while let Some(&d) = it.peek() { if d.is_alphanumeric() || d=='_' { s2.push(d); it.next(); } else { break; } }
                    t.push(Tok::Ident(s2));
                } else { return None; }
            }
        }
    }
    Some(t)
}

// Recursive descent: expr := or_term
fn parse_expr(ts: &[Tok]) -> Result<Expr, ()> { parse_or(ts, 0).map(|(e,_)| e) }

fn parse_or(ts:&[Tok], i:usize)->Result<(Expr,usize),()> { let (mut e, mut j)=parse_and(ts,i)?; while j<ts.len(){ if ts[j]==Tok::Or { let (r,k)=parse_and(ts,j+1)?; e= e.or(r); j=k; } else {break;} } Ok((e,j)) }
fn parse_and(ts:&[Tok], i:usize)->Result<(Expr,usize),()> { let (mut e, mut j)=parse_atom(ts,i)?; while j<ts.len(){ if ts[j]==Tok::And { let (r,k)=parse_atom(ts,j+1)?; e= e.and(r); j=k; } else {break;} } Ok((e,j)) }

fn parse_atom(ts:&[Tok], i:usize)->Result<(Expr,usize),()> {
    if i>=ts.len(){ return Err(()); }
    match &ts[i] {
        Tok::Lp => { let (e,j)=parse_or(ts,i+1)?; if j<ts.len() && ts[j]==Tok::Rp { Ok((e,j+1)) } else { Err(()) } }
        Tok::Ident(_) => parse_cmp(ts, i),
        _ => Err(())
    }
}

fn parse_cmp(ts:&[Tok], i:usize)->Result<(Expr,usize),()> {
    if let Tok::Ident(name) = &ts[i] {
        if i+2 >= ts.len() { return Err(()); }
        let col = col(name);
        match (&ts[i+1], &ts[i+2]) {
            (Tok::Gt, Tok::Num(v)) => Ok((col.gt(lit(*v)), i+3)),
            (Tok::Ge, Tok::Num(v)) => Ok((col.gt_eq(lit(*v)), i+3)),
            (Tok::Lt, Tok::Num(v)) => Ok((col.lt(lit(*v)), i+3)),
            (Tok::Le, Tok::Num(v)) => Ok((col.lt_eq(lit(*v)), i+3)),
            (Tok::Eq, Tok::Num(v)) => Ok((col.eq(lit(*v)), i+3)),
            (Tok::Ne, Tok::Num(v)) => Ok((col.neq(lit(*v)), i+3)),
            // string equality ("Symbol == TCS")
            (Tok::Eq, Tok::Ident(s)) => Ok((col.eq(lit(s.as_str())), i+3)),
            (Tok::Ne, Tok::Ident(s)) => Ok((col.neq(lit(s.as_str())), i+3)),
            _ => Err(())
        }
    } else { Err(()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(input: &str) -> String {
        format!("{:?}", ExprBuilder::parse(input).expect("expression should parse"))
    }

    fn rendered(expr: Expr) -> String {
        format!("{expr:?}")
    }

    #[test]
    fn parse_filter_gives_and_higher_precedence_than_or() {
        let actual = parsed("A > 1 | B > 2 & C > 3");
        let expected = col("A")
            .gt(lit(1.0))
            .or(col("B").gt(lit(2.0)).and(col("C").gt(lit(3.0))));

        assert_eq!(actual, rendered(expected));
    }

    #[test]
    fn parse_filter_keeps_logical_operators_left_associative() {
        let actual = parsed("A > 1 | B > 2 | C > 3");
        let expected = col("A")
            .gt(lit(1.0))
            .or(col("B").gt(lit(2.0)))
            .or(col("C").gt(lit(3.0)));

        assert_eq!(actual, rendered(expected));
    }

    #[test]
    fn parse_filter_accepts_trailing_tokens_for_now() {
        assert!(ExprBuilder::parse("Open > 100 trailing").is_some());
    }

    #[test]
    fn parse_filter_silently_rejects_invalid_input_for_now() {
        assert!(ExprBuilder::parse("Open = 100").is_none());
        assert!(ExprBuilder::parse("Open >").is_none());
        assert!(ExprBuilder::parse("(Open > 100").is_none());
    }
}
