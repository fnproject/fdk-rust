use fdk::{Function, FunctionError, Result, RuntimeContext};
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    if let Err(e) = Function::run(|_: &mut RuntimeContext, i: String| {
        Ok(format!(
            "Hello {}!",
            if i.is_empty() {
                "world"
            } else {
                i.trim_end_matches("\n")
            }
        ))
    })
    .await
    {
        eprintln!("{}", e);
    }
    Ok(())
}
