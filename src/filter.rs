use crate::numeric::{parse_numeric, NumericError};
use polars::prelude::*;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComparisonOp {
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
    Ne,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterValue {
    Number(f64),
    Text(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterExpr {
    Compare {
        column: String,
        op: ComparisonOp,
        value: FilterValue,
    },
    And(Box<FilterExpr>, Box<FilterExpr>),
    Or(Box<FilterExpr>, Box<FilterExpr>),
}

impl FilterExpr {
    pub fn to_polars_expr(&self) -> Expr {
        match self {
            Self::Compare { column, op, value } => {
                let lhs = col(column);
                match (op, value) {
                    (ComparisonOp::Gt, FilterValue::Number(value)) => lhs.gt(lit(*value)),
                    (ComparisonOp::Ge, FilterValue::Number(value)) => lhs.gt_eq(lit(*value)),
                    (ComparisonOp::Lt, FilterValue::Number(value)) => lhs.lt(lit(*value)),
                    (ComparisonOp::Le, FilterValue::Number(value)) => lhs.lt_eq(lit(*value)),
                    (ComparisonOp::Eq, FilterValue::Number(value)) => lhs.eq(lit(*value)),
                    (ComparisonOp::Ne, FilterValue::Number(value)) => lhs.neq(lit(*value)),
                    (ComparisonOp::Eq, FilterValue::Text(value)) => lhs.eq(lit(value.as_str())),
                    (ComparisonOp::Ne, FilterValue::Text(value)) => lhs.neq(lit(value.as_str())),
                    _ => unreachable!("invalid comparison produced by parser"),
                }
            }
            Self::And(left, right) => left.to_polars_expr().and(right.to_polars_expr()),
            Self::Or(left, right) => left.to_polars_expr().or(right.to_polars_expr()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterParseError {
    EmptyInput,
    UnexpectedToken {
        found: String,
        position: usize,
    },
    UnexpectedEnd {
        position: usize,
    },
    TrailingTokens {
        position: usize,
    },
    InvalidNumber(NumericError),
    InvalidComparison {
        operator: ComparisonOp,
        literal: String,
        position: usize,
    },
    UnclosedGroup {
        position: usize,
    },
}

impl fmt::Display for FilterParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(f, "filter expression is empty"),
            Self::UnexpectedToken { found, position } => {
                write!(f, "unexpected token {found:?} at byte {position}")
            }
            Self::UnexpectedEnd { position } => {
                write!(f, "unexpected end of filter at byte {position}")
            }
            Self::TrailingTokens { position } => {
                write!(f, "filter expression has trailing tokens at byte {position}")
            }
            Self::InvalidNumber(error) => write!(f, "invalid numeric literal: {error}"),
            Self::InvalidComparison {
                operator,
                literal,
                position,
            } => write!(
                f,
                "operator {operator:?} cannot compare against literal {literal:?} at byte {position}"
            ),
            Self::UnclosedGroup { position } => {
                write!(f, "unclosed parenthesized group starting at byte {position}")
            }
        }
    }
}

impl std::error::Error for FilterParseError {}

#[derive(Clone, Debug, PartialEq)]
enum TokenKind {
    Ident(String),
    Number(String),
    Gt,
    Ge,
    Lt,
    Le,
    Eq,
    Ne,
    And,
    Or,
    LParen,
    RParen,
}

#[derive(Clone, Debug, PartialEq)]
struct Token {
    kind: TokenKind,
    start: usize,
    end: usize,
}

pub fn parse_filter(input: &str) -> Result<FilterExpr, FilterParseError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(FilterParseError::EmptyInput);
    }
    let mut parser = Parser {
        tokens: &tokens,
        index: 0,
    };
    let expr = parser.parse_or()?;
    if let Some(token) = parser.peek() {
        return Err(FilterParseError::TrailingTokens {
            position: token.start,
        });
    }
    Ok(expr)
}

fn tokenize(input: &str) -> Result<Vec<Token>, FilterParseError> {
    let mut tokens = Vec::new();
    let bytes = input.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }

        let start = index;
        let kind = match byte {
            b'(' => {
                index += 1;
                TokenKind::LParen
            }
            b')' => {
                index += 1;
                TokenKind::RParen
            }
            b'&' => {
                index += 1;
                TokenKind::And
            }
            b'|' => {
                index += 1;
                TokenKind::Or
            }
            b'>' => {
                index += 1;
                if bytes.get(index) == Some(&b'=') {
                    index += 1;
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            b'<' => {
                index += 1;
                if bytes.get(index) == Some(&b'=') {
                    index += 1;
                    TokenKind::Le
                } else {
                    TokenKind::Lt
                }
            }
            b'=' => {
                index += 1;
                if bytes.get(index) == Some(&b'=') {
                    index += 1;
                    TokenKind::Eq
                } else {
                    return Err(FilterParseError::UnexpectedToken {
                        found: "=".to_string(),
                        position: start,
                    });
                }
            }
            b'!' => {
                index += 1;
                if bytes.get(index) == Some(&b'=') {
                    index += 1;
                    TokenKind::Ne
                } else {
                    return Err(FilterParseError::UnexpectedToken {
                        found: "!".to_string(),
                        position: start,
                    });
                }
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => {
                let end = consume_number(bytes, index);
                let text = String::from_utf8(bytes[index..end].to_vec())
                    .expect("numeric token must be valid ASCII");
                index = end;
                TokenKind::Number(text)
            }
            _ if byte.is_ascii_alphabetic() || byte == b'_' => {
                index += 1;
                while bytes
                    .get(index)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    index += 1;
                }
                let text = String::from_utf8(bytes[start..index].to_vec())
                    .expect("identifier token must be valid ASCII");
                TokenKind::Ident(text)
            }
            _ => {
                return Err(FilterParseError::UnexpectedToken {
                    found: (byte as char).to_string(),
                    position: start,
                });
            }
        };
        tokens.push(Token {
            kind,
            start,
            end: index,
        });
    }

    Ok(tokens)
}

fn consume_number(bytes: &[u8], mut index: usize) -> usize {
    if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
        index += 1;
    }
    while matches!(bytes.get(index), Some(b'0'..=b'9')) {
        index += 1;
    }
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
    }
    if matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
            index += 1;
        }
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
    }
    index
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a Token> {
        self.tokens.get(self.index)
    }

    fn next(&mut self) -> Option<&'a Token> {
        let token = self.tokens.get(self.index);
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    fn parse_or(&mut self) -> Result<FilterExpr, FilterParseError> {
        let mut expr = self.parse_and()?;
        while matches!(self.peek().map(|token| &token.kind), Some(TokenKind::Or)) {
            self.next();
            let rhs = self.parse_and()?;
            expr = FilterExpr::Or(Box::new(expr), Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<FilterExpr, FilterParseError> {
        let mut expr = self.parse_atom()?;
        while matches!(self.peek().map(|token| &token.kind), Some(TokenKind::And)) {
            self.next();
            let rhs = self.parse_atom()?;
            expr = FilterExpr::And(Box::new(expr), Box::new(rhs));
        }
        Ok(expr)
    }

    fn parse_atom(&mut self) -> Result<FilterExpr, FilterParseError> {
        let token = self
            .peek()
            .ok_or(FilterParseError::UnexpectedEnd { position: 0 })?;
        match &token.kind {
            TokenKind::LParen => {
                let start = token.start;
                self.next();
                let expr = self.parse_or()?;
                match self.next() {
                    Some(token) if matches!(token.kind, TokenKind::RParen) => Ok(expr),
                    Some(token) => Err(FilterParseError::UnexpectedToken {
                        found: token_text(token),
                        position: token.start,
                    }),
                    None => Err(FilterParseError::UnclosedGroup { position: start }),
                }
            }
            TokenKind::Ident(_) => self.parse_comparison(),
            _ => Err(FilterParseError::UnexpectedToken {
                found: token_text(token),
                position: token.start,
            }),
        }
    }

    fn parse_comparison(&mut self) -> Result<FilterExpr, FilterParseError> {
        let column = match self.next() {
            Some(Token {
                kind: TokenKind::Ident(column),
                ..
            }) => column.clone(),
            Some(token) => {
                return Err(FilterParseError::UnexpectedToken {
                    found: token_text(token),
                    position: token.start,
                })
            }
            None => {
                return Err(FilterParseError::UnexpectedEnd { position: 0 });
            }
        };

        let op_token = self
            .next()
            .ok_or(FilterParseError::UnexpectedEnd { position: 0 })?;
        let op = match op_token.kind {
            TokenKind::Gt => ComparisonOp::Gt,
            TokenKind::Ge => ComparisonOp::Ge,
            TokenKind::Lt => ComparisonOp::Lt,
            TokenKind::Le => ComparisonOp::Le,
            TokenKind::Eq => ComparisonOp::Eq,
            TokenKind::Ne => ComparisonOp::Ne,
            _ => {
                return Err(FilterParseError::UnexpectedToken {
                    found: token_text(op_token),
                    position: op_token.start,
                })
            }
        };

        let value_token = self
            .next()
            .ok_or(FilterParseError::UnexpectedEnd { position: op_token.end })?;
        let value = match &value_token.kind {
            TokenKind::Number(text) => FilterValue::Number(
                parse_numeric(text).map_err(FilterParseError::InvalidNumber)?,
            ),
            TokenKind::Ident(text) => {
                if matches!(op, ComparisonOp::Gt | ComparisonOp::Ge | ComparisonOp::Lt | ComparisonOp::Le)
                {
                    return Err(FilterParseError::InvalidComparison {
                        operator: op,
                        literal: text.clone(),
                        position: value_token.start,
                    });
                }
                FilterValue::Text(text.clone())
            }
            _ => {
                return Err(FilterParseError::UnexpectedToken {
                    found: token_text(value_token),
                    position: value_token.start,
                })
            }
        };

        Ok(FilterExpr::Compare { column, op, value })
    }
}

fn token_text(token: &Token) -> String {
    match &token.kind {
        TokenKind::Ident(text) | TokenKind::Number(text) => text.clone(),
        TokenKind::Gt => ">".to_string(),
        TokenKind::Ge => ">=".to_string(),
        TokenKind::Lt => "<".to_string(),
        TokenKind::Le => "<=".to_string(),
        TokenKind::Eq => "==".to_string(),
        TokenKind::Ne => "!=".to_string(),
        TokenKind::And => "&".to_string(),
        TokenKind::Or => "|".to_string(),
        TokenKind::LParen => "(".to_string(),
        TokenKind::RParen => ")".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_precedence_and_full_consumption() {
        let expr = parse_filter("A > 1 | B > 2 & C > 3").unwrap();
        assert!(matches!(expr, FilterExpr::Or(_, _)));
        assert!(matches!(
            parse_filter("A > 1 trailing"),
            Err(FilterParseError::TrailingTokens { .. })
        ));
    }

    #[test]
    fn rejects_bad_comparisons_and_parentheses() {
        assert!(matches!(
            parse_filter("Open > Close"),
            Err(FilterParseError::InvalidComparison { .. })
        ));
        assert!(matches!(
            parse_filter("(Open > 1"),
            Err(FilterParseError::UnclosedGroup { .. })
        ));
    }
}
