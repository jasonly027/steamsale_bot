#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize)]
pub struct Junction {
    pub channel_id: i64,
    pub sale_threshold: i32,
    pub server_id: i64,
}
