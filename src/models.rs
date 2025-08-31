use derivative::Derivative;
use mongodb::bson;

use crate::steam;

#[derive(Debug, Clone, Default, Derivative, serde::Deserialize, serde::Serialize)]
#[derivative(PartialEq, Eq)]
pub struct Discord {
    #[serde(rename = "_id", skip_serializing)]
    #[derivative(PartialEq = "ignore")]
    pub id: bson::oid::ObjectId,
    pub channel_id: i64,
    pub sale_threshold: i32,
    pub server_id: i64,
}

#[derive(Debug, Clone, Default, Derivative, serde::Deserialize, serde::Serialize)]
#[derivative(PartialEq, Eq)]
pub struct Junction {
    #[serde(rename = "_id", skip_serializing)]
    #[derivative(PartialEq = "ignore")]
    pub id: bson::oid::ObjectId,
    pub app_id: i32,
    pub server_id: i64,
    pub is_trailing_sale_day: bool,
    pub coming_soon: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sale_threshold: Option<i32>,
}

#[derive(Debug, Clone, Default, Derivative, serde::Deserialize, serde::Serialize)]
#[derivative(PartialEq, Eq)]
pub struct App {
    #[serde(rename = "_id", skip_serializing)]
    #[derivative(PartialEq = "ignore")]
    pub id: bson::oid::ObjectId,
    pub app_id: i32,
    pub app_name: String,
}

impl From<steam::App> for App {
    fn from(app: steam::App) -> Self {
        Self {
            id: Default::default(),
            app_id: app.app_id,
            app_name: app.name,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppListing {
    pub app_id: i32,
    pub app_name: String,
    pub sale_threshold: Option<i32>,
}
