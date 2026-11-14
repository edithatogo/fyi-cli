//! The v1 Python float notation, retaining Serde JSON's shortest decimal digits.
//! Serde owns tie rounding, string escaping, containers and integer serialization.
use serde::Serialize;
use serde_json::{ser::Formatter, Serializer, Value};
use std::io::{self, Write};

pub(super) fn to_vec(value: &Value) -> serde_json::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut serializer = Serializer::with_formatter(&mut bytes, PythonFloatFormatter);
    value.serialize(&mut serializer)?;
    Ok(bytes)
}

struct PythonFloatFormatter;

impl Formatter for PythonFloatFormatter {
    fn write_f64<W>(&mut self, writer: &mut W, value: f64) -> io::Result<()>
    where
        W: ?Sized + Write,
    {
        if !value.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "nonfinite receipt number",
            ));
        }
        // Use the default Serde formatter, NOT this receipt formatter. Rust's
        // LowerExp shortest conversion uses a different decimal midpoint tie
        // rule (rust-lang/rust#130936). Reuse Serde's digits; only change notation.
        let rendered = serde_json::to_string(&value).map_err(io::Error::other)?;
        let (sign, unsigned) = match rendered.strip_prefix('-') {
            Some(unsigned) => ("-", unsigned),
            None => ("", rendered.as_str()),
        };
        let (mantissa, scale) = match unsigned.split_once('e') {
            Some((mantissa, exponent)) => (
                mantissa,
                exponent
                    .parse::<i32>()
                    .map_err(|_| io::Error::other("invalid numeric exponent"))?,
            ),
            None => (unsigned, 0),
        };
        let point = mantissa.find('.').unwrap_or(mantissa.len()) as i32;
        let digits = mantissa.replace('.', "");
        let significant = digits.trim_start_matches('0');
        if significant.is_empty() {
            return write!(writer, "{sign}0.0");
        }
        let leading_zeros = (digits.len() - significant.len()) as i32;
        let exponent = scale + point - leading_zeros - 1;
        let digits = significant.trim_end_matches('0');
        writer.write_all(sign.as_bytes())?;
        if !(-4..16).contains(&exponent) {
            let (first, rest) = digits.split_at(1);
            writer.write_all(first.as_bytes())?;
            if !rest.is_empty() {
                write!(writer, ".{rest}")?;
            }
            return write!(writer, "e{exponent:+03}");
        }
        let point = exponent + 1;
        if point <= 0 {
            write!(writer, "0.{}{digits}", "0".repeat((-point) as usize))
        } else {
            let point = point as usize;
            if point >= digits.len() {
                write!(writer, "{digits}{}.0", "0".repeat(point - digits.len()))
            } else {
                let (whole, fraction) = digits.split_at(point);
                write!(writer, "{whole}.{fraction}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nonfinite_numbers_fail_without_writing_bytes() {
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut bytes = Vec::new();
            assert!(PythonFloatFormatter.write_f64(&mut bytes, value).is_err());
            assert!(bytes.is_empty());
        }
    }
}
