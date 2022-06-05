// Copyright (C) 2020-2022 The apca Developers
// SPDX-License-Identifier: GPL-3.0-or-later

use apca::data::v2::trades;
use apca::ApiInfo;
use apca::Client;

use chrono::{DateTime};

use std::str::FromStr;

#[tokio::main]
async fn main() {
  // Requires the following environment variables to be present:
  // - APCA_API_KEY_ID -> your API key
  // - APCA_API_SECRET_KEY -> your secret key
  //
  // Optionally, the following variable is honored:
  // - APCA_API_BASE_URL -> the API base URL to use (set to
  //   https://api.alpaca.markets for live trading)
  let api_info = ApiInfo::from_env().unwrap();
  let client = Client::new(api_info);

  let start = DateTime::from_str("2018-12-03T21:47:00Z").unwrap();
  let end = DateTime::from_str("2018-12-03T21:48:00Z").unwrap();

  // Create request for a limit order for AAPL with a limit price of USD
  // 100.
  let request = trades::TradesReqInit {
    limit : Some(4),
    ..Default::default()
  }
  // We want to go long on AAPL, buying a single share.
  .init("AAPL", start, end);

  let trades = client.issue::<trades::Get>(&request).await.unwrap();
  for t in trades.trades {
    println!("{:?}", t);
  }
}
