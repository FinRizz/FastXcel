use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NumericError {
    Empty,
    InvalidCharacter,
    InvalidSyntax,
    Overflow,
    NotFinite,
}

impl fmt::Display for NumericError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "empty numeric literal"),
            Self::InvalidCharacter => write!(f, "numeric literal contains an invalid character"),
            Self::InvalidSyntax => write!(f, "numeric literal has invalid syntax"),
            Self::Overflow => write!(f, "numeric literal overflows finite f64 range"),
            Self::NotFinite => write!(f, "numeric literal is not finite"),
        }
    }
}

impl std::error::Error for NumericError {}

pub fn parse_numeric(input: &str) -> Result<f64, NumericError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(NumericError::Empty);
    }
    if !text.is_ascii() {
        return Err(NumericError::InvalidCharacter);
    }
    if text.chars().any(|c| matches!(c, ',' | '$' | '_')) {
        return Err(NumericError::InvalidCharacter);
    }
    if text.eq_ignore_ascii_case("nan")
        || text.eq_ignore_ascii_case("inf")
        || text.eq_ignore_ascii_case("infinity")
    {
        return Err(NumericError::NotFinite);
    }

    validate_numeric_syntax(text)?;
    let value = text.parse::<f64>().map_err(|_| NumericError::Overflow)?;
    if !value.is_finite() {
        return Err(NumericError::Overflow);
    }
    Ok(value)
}

fn validate_numeric_syntax(text: &str) -> Result<(), NumericError> {
    let bytes = text.as_bytes();
    let mut index = 0;

    if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
        index += 1;
    }

    let mut integer_digits = 0;
    while matches!(bytes.get(index), Some(b'0'..=b'9')) {
        integer_digits += 1;
        index += 1;
    }

    let mut fraction_digits = 0;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            fraction_digits += 1;
            index += 1;
        }
    }

    if integer_digits == 0 && fraction_digits == 0 {
        return Err(NumericError::InvalidSyntax);
    }

    if matches!(bytes.get(index), Some(b'e') | Some(b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+') | Some(b'-')) {
            index += 1;
        }
        let exponent_start = index;
        while matches!(bytes.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }
        if exponent_start == index {
            return Err(NumericError::InvalidSyntax);
        }
    }

    if index != bytes.len() {
        return Err(NumericError::InvalidSyntax);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_signed_decimals_and_exponents() {
        assert_eq!(parse_numeric(" -12.5 "), Ok(-12.5));
        assert_eq!(parse_numeric("+1e3"), Ok(1000.0));
        assert_eq!(parse_numeric(".5"), Ok(0.5));
    }

    #[test]
    fn rejects_separators_currency_and_nonfinite_values() {
        assert_eq!(parse_numeric("1,200"), Err(NumericError::InvalidCharacter));
        assert_eq!(parse_numeric("$120"), Err(NumericError::InvalidCharacter));
        assert_eq!(parse_numeric("NaN"), Err(NumericError::NotFinite));
        assert_eq!(parse_numeric("inf"), Err(NumericError::NotFinite));
    }

    #[test]
    fn rejects_overflow_and_trailing_tokens() {
        assert_eq!(parse_numeric("1e400"), Err(NumericError::Overflow));
        assert_eq!(
            parse_numeric("12 trailing"),
            Err(NumericError::InvalidSyntax)
        );
    }
}
