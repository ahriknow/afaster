//! 多人博客系统

mod app;
mod bootstrap;
mod model;
mod rbac;
mod service;

#[tokio::main]
async fn main() {
    app::run().await;
}
