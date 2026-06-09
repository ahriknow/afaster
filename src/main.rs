use afaster::AFaster;

#[tokio::main]
async fn main() {
    AFaster::new("config.toml".to_string())
        .await
        .expect("初始化失败")
        .run()
        .await;
}
