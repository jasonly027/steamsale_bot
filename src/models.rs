use mongodb::bson;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Discord {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId,
    pub channel_id: i64,
    pub sale_threshold: i32,
    pub server_id: i64,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct Junction {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId,
    pub app_id: i32,
    pub server_id: i64,
    pub is_trailing_sale_day: bool,
    pub coming_soon: bool,
    pub sale_threshold: Option<i32>,
}

#[derive(Debug, Clone, Default, serde::Deserialize, serde::Serialize)]
pub struct App {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId,
    pub app_id: i32,
    pub app_name: String,
}

#[derive(Debug, Clone, Default)]
pub struct AppListing {
    pub app_id: i32,
    pub app_name: String,
    pub sale_threshold: Option<i32>,
}
