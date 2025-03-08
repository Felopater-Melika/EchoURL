use anyhow::Context;
use entity::url;
use entity::Expr;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use shared::{connect_db, DbPool};
use std::sync::Arc;
use tokio_stream::StreamExt;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let db = connect_db()
        .await
        .context("Database connection failed")
        .unwrap();

    let db = Arc::new(db);

    consume_clicks(db).await;
}

async fn consume_clicks(db: Arc<DbPool>) {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092")
        .set("group.id", "analytics_group")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Kafka consumer creation failed");

    consumer
        .subscribe(&["url_clicks"])
        .expect("Failed to subscribe");

    let mut message_stream = consumer.stream();
    while let Some(Ok(message)) = message_stream.next().await {
        if let Some(payload) = message.payload_view::<str>() {
            if let Ok(payload) = payload {
                if let Some(slug) = extract_slug(payload) {
                    increment_click_count(&db, &slug).await;
                }
            }
        }
    }
}

fn extract_slug(payload: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(payload).ok()?;
    json.get("slug")?.as_str().map(|s| s.to_string())
}

async fn increment_click_count(db: &sea_orm::DatabaseConnection, slug: &str) {
    if let Err(e) = url::Entity::update_many()
        .filter(url::Column::Shortened.eq(slug))
        .col_expr(url::Column::Clicks, Expr::col(url::Column::Clicks).add(1))
        .exec(db)
        .await
    {
        error!("Failed to increment click count for `{}`: {:?}", slug, e);
    } else {
        info!("Incremented click count for `{}`", slug);
    }
}
