mod hello;
mod items;

use rustspring::Application;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Application::new()
        .manage(hello::HelloService::new()) // singleton service (@Bean)
        .component::<items::ItemService>() // constructor-injected (@Component)
        .routes(hello::routes())
        .routes(items::routes())
        .run()
        .await
}
