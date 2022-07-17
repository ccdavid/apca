// Copyright (C) 2021-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use chrono::DateTime;
use chrono::Utc;

use num_decimal::Num;

use serde::Deserialize;
use serde::Serialize;
use serde_urlencoded::to_string as to_query;

use crate::data::v2::Feed;
use crate::data::DATA_BASE_URL;
use crate::util::vec_from_str;
use crate::Str;

/// The symbol.
pub type Symbol = Str;

/// A GET request to be issued to the /v2/stocks/<symbol>/trades endpoint.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TradesReq {
  /// The symbol for which to retrieve market data.
  #[serde(skip)]
  pub symbol: Symbol,
  /// Filter trades equal to or after this time.
  #[serde(rename = "start")]
  pub start: DateTime<Utc>,
  /// Filter trades equal to or before this time.
  #[serde(rename = "end")]
  pub end: DateTime<Utc>,
  /// The maximum number of trades to be returned for each symbol.
  ///
  /// It can be between 1 and 10000. Defaults to 1000 if the provided
  /// value is None.
  #[serde(rename = "limit")]
  pub limit: Option<usize>,
  /// If provided we will pass a page token to continue where we left off.
  #[serde(rename = "page_token", skip_serializing_if = "Option::is_none")]
  pub page_token: Option<String>,
  /// The data feed to use.
  ///
  /// Defaults to [`IEX`][Feed::IEX] for free users and
  /// [`SIP`][Feed::SIP] for users with an unlimited subscription.
  #[serde(rename = "feed")]
  pub feed: Option<Feed>,
}


/// A helper for initializing [`TradesReq`] objects.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TradesReqInit {
  /// See `TradesReq::limit`.
  pub limit: Option<usize>,
  /// See `TradesReq::feed`.
  pub feed: Option<Feed>,
  /// See `TradesReq::page_token`.
  pub page_token: Option<String>,
  #[doc(hidden)]
  pub _non_exhaustive: (),
}

impl TradesReqInit {
  /// Create a [`TradesReq`] from a `TradesReqInit`.
  #[inline]
  pub fn init<S>(
    self,
    symbol: S,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
  ) -> TradesReq
  where
    S: Into<Symbol>,
  {
    TradesReq {
      symbol: symbol.into(),
      start,
      end,
      limit: self.limit,
      page_token: self.page_token,
      feed: self.feed,
    }
  }
}

/// A market data trade as returned by the /v2/stocks/<symbol>/trades endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct Trade {
  /// Timestamp in RFC-3339 format with nanosecond precision.
  #[serde(rename = "t")]
  pub timestamp: DateTime<Utc>,
  /// The exchange where the trade happened.
  #[serde(rename = "x")]
  pub exchange: String,
  #[serde(rename = "p")]
  /// The trade's price.
  pub price: Num,
  /// The trade's size.
  #[serde(rename = "s")]
  pub size: u64,
  /// Trade ID.
  #[serde(rename = "i")]
  pub trade_id: u64,
}

/// A collection of trades as returned by the API. This is one page of trades.
#[derive(Debug, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct Trades {
  /// The list of returned trades.
  #[serde(deserialize_with = "vec_from_str")]
  pub trades: Vec<Trade>,
  /// The symbol the trades correspond to.
  pub symbol: Symbol,
  /// The token to provide to a request to get the next page of trades for this request.
  pub next_page_token: Option<String>,
}

Endpoint! {
  /// The representation of a GET request to the /v2/stocks/<symbol>/trades endpoint.
  pub Get(TradesReq),
  Ok => Trades, [
    /// The market data was retrieved successfully.
    /* 200 */ OK,
  ],
  Err => GetError, [
    /// A query parameter was invalid.
    /* 422 */ UNPROCESSABLE_ENTITY => InvalidInput,
  ]

  fn base_url() -> Option<Str> {
    Some(DATA_BASE_URL.into())
  }

  fn path(input: &Self::Input) -> Str {
    format!("/v2/stocks/{}/trades", input.symbol).into()
  }

  fn query(input: &Self::Input) -> Result<Option<Str>, Self::ConversionError> {
    Ok(Some(to_query(input)?.into()))
  }
}


#[cfg(test)]
mod tests {
  use super::*;

  use std::str::FromStr as _;

  use http_endpoint::Endpoint;

  use serde_json::from_str as from_json;

  use test_log::test;

  use crate::api_info::ApiInfo;
  use crate::Client;
  use crate::RequestError;


  /// Verify that we can properly parse a reference trade response.
  #[test]
  fn parse_reference_trades() {
    let response = r#"{
    "trades": [
      {
        "t": "2022-04-11T12:00:36.002951946Z",
        "x": "V",
        "p": 168.04,
        "s": 50,
        "c": ["@", "T", "I"],
        "i": 1,
        "z": "C"
      },
      {
        "t": "2022-01-11T12:00:36.002951946Z",
        "x": "V",
        "p": 68.04,
        "s": 5,
        "c": ["@", "T", "I"],
        "i": 1,
        "z": "C"
      }
    ],
    "symbol": "AAPL",
    "next_page_token": "QUFQTHwyMDIyLTA0LTExVDEyOjAwOjM2LjAwMjk1MTk0Nlp8VnwwOTIyMzM3MjAzNjg1NDc3NTgwOQ=="
    }"#;

    let res = from_json::<<Get as Endpoint>::Output>(response).unwrap();
    let trades = res.trades;
    let expected_time = DateTime::<Utc>::from_str("2021-02-01T16:01:00Z").unwrap();
    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].timestamp, expected_time);
    assert_eq!(trades[0].exchange, "iex");
    assert_eq!(trades[0].price, Num::new(16804, 100));
    assert_eq!(trades[0].size, 6804);
    assert_eq!(trades[0].trade_id, 1);
    assert_eq!(res.symbol, "AAPL".to_string());
    assert!(res.next_page_token.is_some())
  }

  /// Check that we can decode a response containing no trades correctly.
  #[test(tokio::test)]
  async fn no_trades() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str("2021-11-05T00:00:00Z").unwrap();
    let end = DateTime::from_str("2021-11-05T00:00:00Z").unwrap();
    let request = TradesReqInit::default().init("AAPL", start, end);

    let res = client.issue::<Get>(&request).await.unwrap();
    assert_eq!(res.trades, Vec::new())
  }

  /// Check that we can request historic trade data for a stock.
  #[test(tokio::test)]
  async fn request_trades() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str("2018-12-03T21:47:00Z").unwrap();
    let end = DateTime::from_str("2018-12-06T21:47:00Z").unwrap();
    let request = TradesReqInit {
      limit: Some(2),
      ..Default::default()
    }
    .init("AAPL", start, end);

    let res = client.issue::<Get>(&request).await.unwrap();
    let trades = res.trades;

    assert_eq!(trades.len(), 2);
    assert_eq!(
      trades[0].timestamp,
      DateTime::<Utc>::from_str("2018-12-04T05:00:00Z").unwrap()
    );
    assert_eq!(trades[0].exchange, "iex");
    assert_eq!(trades[0].price, Num::new(17669i32, 100i32));
    assert_eq!(trades[0].size, 3232);
    assert_eq!(
      trades[1].timestamp,
      DateTime::<Utc>::from_str("2018-12-06T05:00:00Z").unwrap()
    );
    assert_eq!(trades[1].exchange, "iex");
  }

  /// Verify that we can request data through a provided page token.
  #[test(tokio::test)]
  async fn can_follow_pagination() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let start = DateTime::from_str("2018-12-03T21:47:00Z").unwrap();
    let end = DateTime::from_str("2018-12-07T21:47:00Z").unwrap();
    let mut request = TradesReqInit {
      limit: Some(2),
      ..Default::default()
    }
    .init("AAPL", start, end);

    let mut res = client.issue::<Get>(&request).await.unwrap();
    let trades = res.trades;

    assert_eq!(trades.len(), 2);
    request.page_token = res.next_page_token;

    res = client.issue::<Get>(&request).await.unwrap();
    let new_trades = res.trades;

    assert_eq!(new_trades.len(), 1);
    assert!(new_trades[0].timestamp > trades[1].timestamp);
    assert!(res.next_page_token.is_none())
  }

  /// Check that we fail as expected when an invalid page token is
  /// specified.
  #[test(tokio::test)]
  async fn invalid_page_token() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2018-12-03T21:47:00Z").unwrap();
    let end = DateTime::from_str("2018-12-07T21:47:00Z").unwrap();
    let request = TradesReqInit {
      page_token: Some("123456789abcdefghi".to_string()),
      ..Default::default()
    }
    .init("SPY", start, end);

    let err = client.issue::<Get>(&request).await.unwrap_err();
    match err {
      RequestError::Endpoint(GetError::InvalidInput(_)) => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }

  /// Verify that we error out as expected when attempting to retrieve
  /// aggregate data trades for a non-existent symbol.
  #[test(tokio::test)]
  async fn nonexistent_symbol() {
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);

    let start = DateTime::from_str("2022-02-01T00:00:00Z").unwrap();
    let end = DateTime::from_str("2022-02-20T00:00:00Z").unwrap();
    let request = TradesReqInit::default().init("ABC123", start, end);

    let err = client.issue::<Get>(&request).await.unwrap_err();
    match err {
      // 42210000 is the error code reported for "invalid symbol".
      RequestError::Endpoint(GetError::InvalidInput(Ok(message))) if message.code == 42210000 => (),
      _ => panic!("Received unexpected error: {:?}", err),
    };
  }
}
