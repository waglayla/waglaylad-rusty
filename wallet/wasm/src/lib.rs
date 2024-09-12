use waglayla_cli_lib::waglayla_cli;
use wasm_bindgen::prelude::*;
use workflow_terminal::Options;
use workflow_terminal::Result;

#[wasm_bindgen]
pub async fn load_waglayla_wallet_cli() -> Result<()> {
    let options = Options { ..Options::default() };
    waglayla_cli(options, None).await?;
    Ok(())
}
