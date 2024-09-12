use crate::result::Result;
use js_sys::BigInt;
use waglayla_consensus_core::network::{NetworkType, NetworkTypeT};
use wasm_bindgen::prelude::*;
use workflow_wasm::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "bigint | number | HexString")]
    #[derive(Clone, Debug)]
    pub type ISompiToWaglayla;
}

/// Convert a Waglayla string to Sompi represented by bigint.
/// This function provides correct precision handling and
/// can be used to parse user input.
/// @category Wallet SDK
#[wasm_bindgen(js_name = "waglaylaToSompi")]
pub fn waglayla_to_sompi(waglayla: String) -> Option<BigInt> {
    crate::utils::try_waglayla_str_to_sompi(waglayla).ok().flatten().map(Into::into)
}

///
/// Convert Sompi to a string representation of the amount in Waglayla.
///
/// @category Wallet SDK
///
#[wasm_bindgen(js_name = "sompiToWaglaylaString")]
pub fn sompi_to_waglayla_string(sompi: ISompiToWaglayla) -> Result<String> {
    let sompi = sompi.try_as_u64()?;
    Ok(crate::utils::sompi_to_waglayla_string(sompi))
}

///
/// Format a Sompi amount to a string representation of the amount in Waglayla with a suffix
/// based on the network type (e.g. `KAS` for mainnet, `TKAS` for testnet,
/// `SKAS` for simnet, `DKAS` for devnet).
///
/// @category Wallet SDK
///
#[wasm_bindgen(js_name = "sompiToWaglaylaStringWithSuffix")]
pub fn sompi_to_waglayla_string_with_suffix(sompi: ISompiToWaglayla, network: &NetworkTypeT) -> Result<String> {
    let sompi = sompi.try_as_u64()?;
    let network_type = NetworkType::try_from(network)?;
    Ok(crate::utils::sompi_to_waglayla_string_with_suffix(sompi, &network_type))
}
