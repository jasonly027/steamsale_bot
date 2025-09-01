//! This module provides integration with the Steam API.

use std::sync::Arc;

use serde::Deserialize;

use crate::StdResult;

/// Error variants when fetching Steam apps using [`Client::fetch_app`].
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Failed to navigate through JSON before deserialization step")]
    MissingJsonField,
    #[error("JSON Deserialization error: {0}")]
    JsonDeserialize(#[from] serde_json::Error),
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct App {
    pub name: String,
    #[serde(rename = "steam_appid")]
    pub app_id: i32,
    pub is_free: bool,
    #[serde(rename = "short_description")]
    pub description: String,
    pub header_image: String,
    pub price_overview: Option<PriceOverview>,
    pub recommendations: Option<Recommendations>,
    pub release_date: ReleaseDate,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct PriceOverview {
    pub discount_percent: i32,
    pub initial_formatted: String,
    pub final_formatted: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Recommendations {
    pub total: u32,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ReleaseDate {
    pub coming_soon: bool,
}

#[derive(Debug, Clone, derivative::Derivative, serde::Deserialize)]
#[derivative(PartialEq, Eq)]
pub struct SearchResult {
    #[serde(rename = "appid", deserialize_with = "str_parse")]
    pub app_id: i32,
    #[derivative(PartialEq = "ignore")]
    pub name: String,
}

/// A client for using the Steam API.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    /// Base url for the store endpoint.
    store_base: Arc<String>,
    /// Base url for the community endpoint.
    community_base: Arc<String>,
}

impl Client {
    pub fn new(store_base: impl Into<String>, community_base: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            store_base: Arc::new(store_base.into()),
            community_base: Arc::new(community_base.into()),
        }
    }

    pub async fn app_details(&self, app_id: i32) -> StdResult<Option<App>, FetchError> {
        let app_id = app_id.to_string();
        let url = format!("{}/api/appdetails", self.store_base);
        let query = [
            (
                "filters",
                "basic,price_overview,recommendations,release_date",
            ),
            ("cc", "US"),
            ("appids", &app_id),
        ];

        let res = self
            .http
            .get(url)
            .query(&query)
            .send()
            .await?
            .error_for_status()?;
        let mut body = res.json::<serde_json::Value>().await?;
        let app_res = body.get_mut(app_id).ok_or(FetchError::MissingJsonField)?;

        let success = app_res
            .get("success")
            .and_then(|s| s.as_bool())
            .ok_or(FetchError::MissingJsonField)?;
        if !success {
            return Ok(None);
        }

        let data = app_res
            .get_mut("data")
            .ok_or(FetchError::MissingJsonField)?
            .take();
        Ok(Some(serde_json::from_value(data)?))
    }

    pub async fn search_apps(&self, query: &str) -> StdResult<Vec<SearchResult>, reqwest::Error> {
        let url = format!(
            "{}/actions/SearchApps/{}",
            self.community_base,
            urlencoding::encode(query)
        );
        self.http.get(url).send().await?.json().await
    }
}

fn str_parse<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let s = String::deserialize(deserializer)?;
    s.parse().map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufRead, Cursor},
        thread,
        time::Duration,
    };

    use pretty_assertions::assert_eq;

    use crate::{
        Result,
        steam::{Client, FetchError},
    };

    fn integration_client() -> Client {
        Client::new(
            "https://store.steampowered.com",
            "https://steamcommunity.com",
        )
    }

    // #[tokio::test]
    // async fn apptest() -> Result<()> {
    //     let client = integration_client();

    //     let data = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/apps_id"));
    //     let cursor = Cursor::new(data);
    //     let lines = cursor.lines();
    //     for (idx, line) in lines.enumerate() {
    //         let app_id: i32 = line?.parse()?;
    //         println!("{idx}. {app_id}");

    //         let app = client.fetch_app(app_id).await;
    //         println!("{app:?}");

    //         if app.is_err_and(|e| {
    //             let FetchError::Http(e) = e else {
    //                 return false;
    //             };
    //             e.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS)
    //         }) {
    //             tokio::time::sleep(Duration::from_secs(60 * 5)).await;
    //         }
    //     }

    //     Ok(())
    // }
}
