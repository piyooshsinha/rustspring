mod greeting;
mod users;

use rustspring::Application;

use crate::greeting::GreetingService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Application::new()
        .manage(GreetingService::new()) // hand-built singleton (@Bean)
        .component::<users::UserService>() // constructor-injected (@Component)
        .routes(greeting::routes())
        .routes(users::routes())
        .run()
        .await
}
