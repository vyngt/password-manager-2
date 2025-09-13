use dotenvy::dotenv;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let vault_db = env::var("DEV_VAULT_DB")?;

    print!("Hello, world! {vault_db}");
    Ok(())
}
