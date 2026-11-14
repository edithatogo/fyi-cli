//! The v1 Python float notation, using Rust's shortest round-trip digits.
//! Serde still owns string escaping, containers and exact integer serialization.
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
        // LowerExp without a precision supplies shortest round-trip digits.
        // Only notation changes: no logarithms, scaling or decimal rounding.
        let scientific = format!("{value:e}");
        let (mantissa, exponent) = scientific
            .split_once('e')
            .ok_or_else(|| io::Error::other("missing numeric exponent"))?;
        let exponent: i32 = exponent
            .parse()
            .map_err(|_| io::Error::other("invalid numeric exponent"))?;
        if !(-4..16).contains(&exponent) {
            return write!(writer, "{mantissa}e{exponent:+03}");
        }
        let (sign, unsigned) = match mantissa.strip_prefix('-') {
            Some(unsigned) => ("-", unsigned),
            None => ("", mantissa),
        };
        let digits = unsigned.replace('.', "");
        let point = exponent + 1;
        writer.write_all(sign.as_bytes())?;
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
