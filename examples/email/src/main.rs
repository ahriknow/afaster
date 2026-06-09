use afaster::email::{Email, EmailConfig};
use serde::Deserialize;

#[derive(Deserialize)]
struct Config {
    email: EmailConfig,
}

#[tokio::main]
async fn main() {
    // 从配置文件构建邮件服务
    let config: Config = toml::from_str(
        &std::fs::read_to_string("examples/email/config.toml").expect("读取 config.toml 失败"),
    )
    .expect("解析 config.toml 失败");

    let email = Email::from_config(&config.email).expect("邮件服务初始化失败");

    // 发送测试邮件（无抄送）
    match email
        .send(
            "test@ahriknow.com",
            "Test",
            "This is a test email from AFaster.",
            &[],
        )
        .await
    {
        Ok(()) => println!("✅ 邮件发送成功"),
        Err(e) => eprintln!("❌ 邮件发送失败: {}", e),
    }
}
