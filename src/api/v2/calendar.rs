// Copyright (C) 2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use std::ops::Range;

use chrono::NaiveDate;
use chrono::NaiveTime;

use serde::de::Error;
use serde::de::Unexpected;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::Str;


/// Deserialize a `NaiveTime` from a string.
fn deserialize_naive_time<'de, D>(deserializer: D) -> Result<NaiveTime, D::Error>
where
  D: Deserializer<'de>,
{
  let string = String::deserialize(deserializer)?;
  NaiveTime::parse_from_str(&string, "%H:%M").map_err(|_| {
    Error::invalid_value(
      Unexpected::Str(&string),
      &"a time stamp string in format %H:%M",
    )
  })
}


/// The market open and close times for a specific date.
#[derive(Clone, Copy, Deserialize, PartialEq, Debug)]
pub struct OpenClose {
  /// The date to which the below open a close times apply.
  #[serde(rename = "date")]
  pub date: NaiveDate,
  /// The time the market opens at.
  #[serde(rename = "open", deserialize_with = "deserialize_naive_time")]
  pub open: NaiveTime,
  /// The time the market closes at.
  #[serde(rename = "close", deserialize_with = "deserialize_naive_time")]
  pub close: NaiveTime,
}


/// A GET request to be made to the /v2/calendar endpoint.
#[derive(Clone, Copy, Serialize, PartialEq, Debug)]
pub struct CalendarReq {
  /// The (inclusive) start date of the range for which to retrieve
  /// calendar data.
  #[serde(rename = "start")]
  pub start: NaiveDate,
  /// The (exclusive) end date of the range for which to retrieve
  /// calendar data.
  // Note that Alpaca claims that the end date is inclusive as well. It
  // is not.
  #[serde(rename = "end")]
  pub end: NaiveDate,
}

impl From<Range<NaiveDate>> for CalendarReq {
  fn from(range: Range<NaiveDate>) -> Self {
    Self {
      start: range.start,
      end: range.end,
    }
  }
}


Endpoint! {
  /// The representation of a GET request to the /v2/calendar endpoint.
  pub Get(CalendarReq),
  Ok => Vec<OpenClose>, [
    /// The market open and close times were retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, []

  fn path(_input: &Self::Input) -> Str {
    "/v2/calendar".into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use crate::api_info::ApiInfo;
  use crate::Client;

  use serde_json::from_str as from_json;

  use test_log::test;


  /// Check that we error out as expected when failing to parse an
  /// `OpenClose` object because the time format is unexpected.
  #[test]
  fn parse_open_close() {
    let serialized = r#"{"date":"2020-04-09","open":"09:30","close":"16:00"}"#;
    let open_close = from_json::<OpenClose>(serialized).unwrap();
    let expected = OpenClose {
      date: NaiveDate::from_ymd(2020, 4, 9),
      open: NaiveTime::from_hms(9, 30, 0),
      close: NaiveTime::from_hms(16, 0, 0),
    };
    assert_eq!(open_close, expected);
  }

  /// Check that we error out as expected when failing to parse an
  /// `OpenClose` object because the time format is unexpected.
  #[test]
  fn parse_open_close_unexpected_time() {
    let serialized = r#"{"date":"2020-04-09","open":"09:30:00","close":"16:00"}"#;
    let err = from_json::<OpenClose>(serialized).unwrap_err();
    assert!(err
      .to_string()
      .starts_with("invalid value: string \"09:30:00\""));
  }

  /// Check that we can retrieve the market calendar for a specific time
  /// frame.
  #[test(tokio::test)]
  async fn get() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = NaiveDate::from_ymd(2020, 4, 6);
    let end = NaiveDate::from_ymd(2020, 4, 10);
    let calendar = client
      .issue::<Get>(&CalendarReq::from(start..end))
      .await
      .unwrap();

    let expected = (6..10)
      .map(|day| OpenClose {
        date: NaiveDate::from_ymd(2020, 4, day),
        open: NaiveTime::from_hms(9, 30, 0),
        close: NaiveTime::from_hms(16, 0, 0),
      })
      .collect::<Vec<_>>();

    assert_eq!(calendar, expected);
  }
}
