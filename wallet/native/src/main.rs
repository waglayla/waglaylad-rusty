use waglayla_cli_lib::{waglayla_cli, TerminalOptions};

#[tokio::main]
async fn main() {
    let result = waglayla_cli(TerminalOptions::new().with_prompt("$ "), None).await;
    if let Err(err) = result {
        println!("{err}");
    }
}
