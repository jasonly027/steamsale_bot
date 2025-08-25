use mongodb::bson;

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Discord {
    #[serde(rename = "_id")]
    pub id: bson::oid::ObjectId,
    pub channel_id: i64,
    pub sale_threshold: i32,
    pub server_id: i64,
}
