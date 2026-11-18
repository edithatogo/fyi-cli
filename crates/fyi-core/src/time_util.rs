//! Internal `time` formatting, parsing, and serde helpers preserving exact wire formats.

use time::format_description::well_known::Rfc3339;
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{Date, OffsetDateTime, UtcOffset};

const DATE_FORMAT: &[FormatItem<'_>] = format_description!("[year]-[month]-[day]");
const MILLIS_Z_FORMAT: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");
const MICROS_Z_FORMAT: &[FormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z");

/// Parse a strict RFC 3339 timestamp requiring a `T`/`t` date-time separator.
pub(crate) fn parse_strict_rfc3339(value: &str) -> Result<OffsetDateTime, String> {
    if !matches!(value.as_bytes().get(10), Some(b'T' | b't')) {
        return Err("expected RFC 3339 'T' separator".to_string());
    }
    OffsetDateTime::parse(value, &Rfc3339).map_err(|error| error.to_string())
}

/// Parse a strict RFC 3339 timestamp and normalize its offset to UTC.
pub(crate) fn parse_strict_rfc3339_utc(value: &str) -> Result<OffsetDateTime, String> {
    parse_strict_rfc3339(value).map(|dt| dt.to_offset(UtcOffset::UTC))
}

/// Format a UTC timestamp as RFC 3339 with `Z` suffix.
pub(crate) fn now_rfc3339_z() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("current UTC timestamp always formats as RFC 3339")
}

/// Format an `OffsetDateTime` in UTC as RFC 3339 with millisecond precision and `Z` suffix.
pub(crate) fn format_rfc3339_millis_z(timestamp: OffsetDateTime) -> String {
    timestamp
        .to_offset(UtcOffset::UTC)
        .format(MILLIS_Z_FORMAT)
        .expect("UTC timestamp always formats with millisecond precision")
}

/// Format current UTC time as RFC 3339 with millisecond precision and `Z` suffix.
pub(crate) fn now_rfc3339_millis_z() -> String {
    format_rfc3339_millis_z(OffsetDateTime::now_utc())
}

/// Format current UTC time as RFC 3339 with microsecond precision and `Z` suffix.
pub(crate) fn now_rfc3339_micros_z() -> String {
    OffsetDateTime::now_utc()
        .format(MICROS_Z_FORMAT)
        .expect("current UTC timestamp always formats with microsecond precision")
}

/// Serde adapter for `time::Date` using strict `YYYY-MM-DD` strings.
pub(crate) mod date {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(value: &Date, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = value
            .format(DATE_FORMAT)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Date, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Date::parse(&raw, DATE_FORMAT).map_err(de::Error::custom)
    }
}

/// Serde adapter for `Option<time::Date>` using `null` or strict `YYYY-MM-DD` strings.
pub(crate) mod option_date {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(value: &Option<Date>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(date) => super::date::serialize(date, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<Date>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<String>::deserialize(deserializer)? {
            Some(raw) => Date::parse(&raw, DATE_FORMAT)
                .map(Some)
                .map_err(de::Error::custom),
            None => Ok(None),
        }
    }
}

/// Serde adapter for `BTreeSet<time::Date>` using sequences of `YYYY-MM-DD` strings.
pub(crate) mod date_btree_set {
    use super::*;
    use serde::de::{self, SeqAccess, Visitor};
    use serde::ser::SerializeSeq;
    use serde::{Deserializer, Serializer};
    use std::collections::BTreeSet;
    use std::fmt;

    pub(crate) fn serialize<S>(values: &BTreeSet<Date>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(values.len()))?;
        for date in values {
            let formatted = date
                .format(DATE_FORMAT)
                .map_err(serde::ser::Error::custom)?;
            seq.serialize_element(&formatted)?;
        }
        seq.end()
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<BTreeSet<Date>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DateSetVisitor;

        impl<'de> Visitor<'de> for DateSetVisitor {
            type Value = BTreeSet<Date>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a sequence of YYYY-MM-DD date strings")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut set = BTreeSet::new();
                while let Some(raw) = seq.next_element::<String>()? {
                    let parsed = Date::parse(&raw, DATE_FORMAT).map_err(de::Error::custom)?;
                    set.insert(parsed);
                }
                Ok(set)
            }
        }

        deserializer.deserialize_seq(DateSetVisitor)
    }
}

/// Serde adapter for UTC `OffsetDateTime` using strict RFC 3339 with `Z` suffix.
pub(crate) mod rfc3339_utc {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(value: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let formatted = value
            .to_offset(UtcOffset::UTC)
            .format(&Rfc3339)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&formatted)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        parse_strict_rfc3339_utc(&raw).map_err(de::Error::custom)
    }
}

/// Serde adapter for `Option<OffsetDateTime>` using strict RFC 3339 in UTC.
pub(crate) mod rfc3339_utc_option {
    use super::*;
    use serde::{de, Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(
        value: &Option<OffsetDateTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(dt) => super::rfc3339_utc::serialize(dt, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<Option<OffsetDateTime>, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<String>::deserialize(deserializer)? {
            Some(raw) => parse_strict_rfc3339_utc(&raw)
                .map(Some)
                .map_err(de::Error::custom),
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeSet;
    use time::macros::{date, datetime};

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Sample {
        #[serde(with = "crate::time_util::date")]
        d: Date,
        #[serde(default, with = "crate::time_util::option_date")]
        opt_d: Option<Date>,
        #[serde(with = "crate::time_util::date_btree_set")]
        dates: BTreeSet<Date>,
        #[serde(with = "crate::time_util::rfc3339_utc")]
        dt: OffsetDateTime,
        #[serde(default, with = "crate::time_util::rfc3339_utc_option")]
        opt_dt: Option<OffsetDateTime>,
    }

    #[test]
    fn strict_rfc3339_and_precision_formatters_preserve_wire_contract() {
        assert!(parse_strict_rfc3339("2026-11-18 08:51:15Z").is_err());
        let dt = parse_strict_rfc3339_utc("2026-11-18t18:51:15.123456+10:00").unwrap();
        assert_eq!(dt, datetime!(2026 - 11 - 18 08:51:15.123456 UTC));
        assert_eq!(format_rfc3339_millis_z(dt), "2026-11-18T08:51:15.123Z");
        assert!(now_rfc3339_z().ends_with('Z'));
        assert!(now_rfc3339_millis_z().ends_with('Z'));
        assert!(now_rfc3339_micros_z().ends_with('Z'));
    }

    #[test]
    fn serde_adapters_roundtrip_and_reject_non_string_dates() {
        let sample = Sample {
            d: date!(2024 - 02 - 29),
            opt_d: Some(date!(2026 - 01 - 31)),
            dates: BTreeSet::from([date!(2026 - 07 - 03)]),
            dt: datetime!(2026 - 11 - 14 15:06:07 +10:00),
            opt_dt: Some(datetime!(2026 - 11 - 14 00:00:00 UTC)),
        };
        let encoded = serde_json::to_value(&sample).unwrap();
        assert_eq!(encoded["d"], "2024-02-29");
        assert_eq!(encoded["opt_d"], "2026-01-31");
        assert_eq!(encoded["dates"], serde_json::json!(["2026-07-03"]));
        assert_eq!(encoded["dt"], "2026-11-14T05:06:07Z");
        assert_eq!(encoded["opt_dt"], "2026-11-14T00:00:00Z");

        let decoded: Sample = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.d, sample.d);
        assert_eq!(decoded.opt_d, sample.opt_d);
        assert_eq!(decoded.dates, sample.dates);
        assert_eq!(decoded.dt, datetime!(2026 - 11 - 14 05:06:07 UTC));
        assert_eq!(decoded.opt_dt, sample.opt_dt);

        let none_sample = Sample {
            d: date!(2026 - 01 - 01),
            opt_d: None,
            dates: BTreeSet::new(),
            dt: datetime!(2026 - 01 - 01 00:00:00 UTC),
            opt_dt: None,
        };
        let none_json = serde_json::to_string(&none_sample).unwrap();
        assert_eq!(
            serde_json::from_str::<Sample>(&none_json).unwrap(),
            none_sample
        );

        // Tuple-encoded dates and space-separated RFC 3339 timestamps must fail closed.
        let bad_date = serde_json::json!({
            "d": [2026, 1],
            "dates": [],
            "dt": "2026-01-01T00:00:00Z"
        });
        assert!(serde_json::from_value::<Sample>(bad_date).is_err());

        let bad_set = serde_json::json!({
            "d": "2026-01-01",
            "dates": "2026-01-01",
            "dt": "2026-01-01T00:00:00Z"
        });
        assert!(serde_json::from_value::<Sample>(bad_set).is_err());
    }
}
