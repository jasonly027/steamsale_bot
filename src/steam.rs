use serde::Deserialize;

use crate::StdResult;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("HTTP error: {0}")]
    Http(reqwest::Error),
    #[error("Failed to navigate through JSON before deserializing step")]
    MissingJsonField,
    #[error("JSON Deserialization error: {0}")]
    JsonDeserialize(serde_json::Error),
}

impl From<reqwest::Error> for FetchError {
    fn from(error: reqwest::Error) -> Self {
        FetchError::Http(error)
    }
}

impl From<serde_json::Error> for FetchError {
    fn from(error: serde_json::Error) -> Self {
        FetchError::JsonDeserialize(error)
    }
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
    app_id: i32,
    #[derivative(PartialEq = "ignore")]
    name: String,
}

#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    store_base: String,
    community_base: String,
}

impl Client {
    pub fn new(store_base: impl Into<String>, community_base: impl Into<String>) -> Self {
        Self {
            http: reqwest::Client::new(),
            store_base: store_base.into(),
            community_base: community_base.into(),
        }
    }

    pub async fn fetch_one_full_app(&self, app_id: i32) -> StdResult<App, FetchError> {
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

        let res = self.http.get(url).query(&query).send().await?;
        let mut body = res.json::<serde_json::Value>().await?;
        let mut data = body
            .get_mut(app_id)
            .ok_or(FetchError::MissingJsonField)?
            .get_mut("data")
            .ok_or(FetchError::MissingJsonField)?
            .take();

        Ok(serde_json::from_value(data.take())?)
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
    use pretty_assertions::assert_eq;

    use crate::{Result, steam::Client};

    fn integration_client() -> Client {
        Client::new(
            "https://store.steampowered.com",
            "https://steamcommunity.com",
        )
    }
}
