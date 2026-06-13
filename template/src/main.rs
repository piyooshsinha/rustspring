mod hello;

use rustspring::Application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Application::new()
        .manage(hello::HelloService::new())
        .routes(hello::routes())
        .run()
        .await
}
