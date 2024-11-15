use crate::model::*;
use borsh::{BorshDeserialize, BorshSerialize};
use waglayla_consensus_core::api::stats::BlockCount;
use waglayla_core::debug;
use waglayla_notify::subscription::{context::SubscriptionContext, single::UtxosChangedSubscription, Command};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    sync::Arc,
};

#[cfg(not(target_family = "wasm"))]
use pyo3::prelude::*;

pub type RpcExtraData = Vec<u8>;

/// SubmitBlockRequest requests to submit a block into the DAG.
/// Blocks are generally expected to have been generated using the getBlockTemplate call.
///
/// See: [`GetBlockTemplateRequest`]
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitBlockRequest {
    pub block: RpcBlock,
    #[serde(alias = "allowNonDAABlocks")]
    pub allow_non_daa_blocks: bool,
}
impl SubmitBlockRequest {
    pub fn new(block: RpcBlock, allow_non_daa_blocks: bool) -> Self {
        Self { block, allow_non_daa_blocks }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(not(target_family = "wasm"), pyclass)]
pub enum SubmitBlockRejectReason {
    BlockInvalid = 1,
    IsInIBD = 2,
    RouteIsFull = 3,
}

impl SubmitBlockRejectReason {
    fn as_str(&self) -> &'static str {
        // see app\appmessage\rpc_submit_block.go, line 35
        match self {
            SubmitBlockRejectReason::BlockInvalid => "block is invalid",
            SubmitBlockRejectReason::IsInIBD => "node is not synced",
            SubmitBlockRejectReason::RouteIsFull => "route is full",
        }
    }
}
impl Display for SubmitBlockRejectReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "lowercase")]
#[serde(tag = "type", content = "reason")]
pub enum SubmitBlockReport {
    Success,
    Reject(SubmitBlockRejectReason),
}
impl SubmitBlockReport {
    pub fn is_success(&self) -> bool {
        *self == SubmitBlockReport::Success
    }
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitBlockResponse {
    pub report: SubmitBlockReport,
}

/// GetBlockTemplateRequest requests a current block template.
/// Callers are expected to solve the block template and submit it using the submitBlock call
///
/// See: [`SubmitBlockRequest`]
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockTemplateRequest {
    /// Which waglayla address should the coinbase block reward transaction pay into
    pub pay_address: RpcAddress,
    // TODO: replace with hex serialization
    pub extra_data: RpcExtraData,
}
impl GetBlockTemplateRequest {
    pub fn new(pay_address: RpcAddress, extra_data: RpcExtraData) -> Self {
        Self { pay_address, extra_data }
    }
}

#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetBlockTemplateResponse {
    #[pyo3(get)]
    pub block: RpcBlock,

    /// Whether waglaylad thinks that it's synced.
    /// Callers are discouraged (but not forbidden) from solving blocks when waglaylad is not synced.
    /// That is because when waglaylad isn't in sync with the rest of the network there's a high
    /// chance the block will never be accepted, thus the solving effort would have been wasted.
    #[pyo3(get)]
    pub is_synced: bool,
}

#[cfg(target_family = "wasm")]
#[derive(Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockTemplateResponse {
    pub block: RpcBlock,

    /// Whether waglaylad thinks that it's synced.
    /// Callers are discouraged (but not forbidden) from solving blocks when waglaylad is not synced.
    /// That is because when waglaylad isn't in sync with the rest of the network there's a high
    /// chance the block will never be accepted, thus the solving effort would have been wasted.
    pub is_synced: bool,
}

/// GetOutpointRequest requests information about a specific outpoint
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOutpointRequest {
    /// The hash of the requested outpoint
    pub hash: RpcHash,
}

impl GetOutpointRequest {
    pub fn new(hash: RpcHash) -> Self {
        Self { hash }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOutpointResponse {
    pub entry: RpcUtxoEntry,
}

/// GetBlockRequest requests information about a specific block
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockRequest {
    /// The hash of the requested block
    pub hash: RpcHash,

    /// Whether to include transaction data in the response
    pub include_transactions: bool,
}
impl GetBlockRequest {
    pub fn new(hash: RpcHash, include_transactions: bool) -> Self {
        Self { hash, include_transactions }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockResponse {
    pub block: RpcBlock,
}

/// GetInfoRequest returns info about the node.
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInfoRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetInfoResponse {
    #[pyo3(get)]
    pub p2p_id: String,
    #[pyo3(get)]
    pub mempool_size: u64,
    #[pyo3(get)]
    pub server_version: String,
    #[pyo3(get)]
    pub is_utxo_indexed: bool,
    #[pyo3(get)]
    pub is_synced: bool,
    #[pyo3(get)]
    pub has_notify_command: bool,
    #[pyo3(get)]
    pub has_message_id: bool,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInfoResponse {
    pub p2p_id: String,
    pub mempool_size: u64,
    pub server_version: String,
    pub is_utxo_indexed: bool,
    pub is_synced: bool,
    pub has_notify_command: bool,
    pub has_message_id: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentNetworkRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCurrentNetworkResponse {
    pub network: RpcNetworkType,
}

impl GetCurrentNetworkResponse {
    pub fn new(network: RpcNetworkType) -> Self {
        Self { network }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPeerAddressesRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetPeerAddressesResponse {
    #[pyo3(get)]
    pub known_addresses: Vec<RpcPeerAddress>,
    #[pyo3(get)]
    pub banned_addresses: Vec<RpcIpAddress>,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPeerAddressesResponse {
    pub known_addresses: Vec<RpcPeerAddress>,
    pub banned_addresses: Vec<RpcIpAddress>,
}

impl GetPeerAddressesResponse {
    pub fn new(known_addresses: Vec<RpcPeerAddress>, banned_addresses: Vec<RpcIpAddress>) -> Self {
        Self { known_addresses, banned_addresses }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSinkRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetSinkResponse {
    #[pyo3(get)]
    pub sink: RpcHash,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSinkResponse {
    pub sink: RpcHash,
}

impl GetSinkResponse {
    pub fn new(selected_tip_hash: RpcHash) -> Self {
        Self { sink: selected_tip_hash }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMempoolEntryRequest {
    pub transaction_id: RpcTransactionId,
    pub include_orphan_pool: bool,
    // TODO: replace with `include_transaction_pool`
    pub filter_transaction_pool: bool,
}

impl GetMempoolEntryRequest {
    pub fn new(transaction_id: RpcTransactionId, include_orphan_pool: bool, filter_transaction_pool: bool) -> Self {
        Self { transaction_id, include_orphan_pool, filter_transaction_pool }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMempoolEntryResponse {
    pub mempool_entry: RpcMempoolEntry,
}

impl GetMempoolEntryResponse {
    pub fn new(mempool_entry: RpcMempoolEntry) -> Self {
        Self { mempool_entry }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMempoolEntriesRequest {
    pub include_orphan_pool: bool,
    // TODO: replace with `include_transaction_pool`
    pub filter_transaction_pool: bool,
}

impl GetMempoolEntriesRequest {
    pub fn new(include_orphan_pool: bool, filter_transaction_pool: bool) -> Self {
        Self { include_orphan_pool, filter_transaction_pool }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMempoolEntriesResponse {
    pub mempool_entries: Vec<RpcMempoolEntry>,
}

impl GetMempoolEntriesResponse {
    pub fn new(mempool_entries: Vec<RpcMempoolEntry>) -> Self {
        Self { mempool_entries }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConnectedPeerInfoRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetConnectedPeerInfoResponse {
    #[pyo3(get)]
    pub peer_info: Vec<RpcPeerInfo>,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConnectedPeerInfoResponse {
    pub peer_info: Vec<RpcPeerInfo>,
}

impl GetConnectedPeerInfoResponse {
    pub fn new(peer_info: Vec<RpcPeerInfo>) -> Self {
        Self { peer_info }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPeerRequest {
    pub peer_address: RpcContextualPeerAddress,
    pub is_permanent: bool,
}

impl AddPeerRequest {
    pub fn new(peer_address: RpcContextualPeerAddress, is_permanent: bool) -> Self {
        Self { peer_address, is_permanent }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPeerResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTransactionRequest {
    pub transaction: RpcTransaction,
    pub allow_orphan: bool,
}

impl SubmitTransactionRequest {
    pub fn new(transaction: RpcTransaction, allow_orphan: bool) -> Self {
        Self { transaction, allow_orphan }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitTransactionResponse {
    pub transaction_id: RpcTransactionId,
}

impl SubmitTransactionResponse {
    pub fn new(transaction_id: RpcTransactionId) -> Self {
        Self { transaction_id }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSubnetworkRequest {
    pub subnetwork_id: RpcSubnetworkId,
}

impl GetSubnetworkRequest {
    pub fn new(subnetwork_id: RpcSubnetworkId) -> Self {
        Self { subnetwork_id }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetSubnetworkResponse {
    #[pyo3(get)]
    pub gas_limit: u64,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSubnetworkResponse {
    pub gas_limit: u64,
}

impl GetSubnetworkResponse {
    pub fn new(gas_limit: u64) -> Self {
        Self { gas_limit }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVirtualChainFromBlockRequest {
    pub start_hash: RpcHash,
    pub include_accepted_transaction_ids: bool,
}

impl GetVirtualChainFromBlockRequest {
    pub fn new(start_hash: RpcHash, include_accepted_transaction_ids: bool) -> Self {
        Self { start_hash, include_accepted_transaction_ids }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetVirtualChainFromBlockResponse {
    #[pyo3(get)]
    pub removed_chain_block_hashes: Vec<RpcHash>,
    #[pyo3(get)]
    pub added_chain_block_hashes: Vec<RpcHash>,
    #[pyo3(get)]
    pub accepted_transaction_ids: Vec<RpcAcceptedTransactionIds>,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVirtualChainFromBlockResponse {
    pub removed_chain_block_hashes: Vec<RpcHash>,
    pub added_chain_block_hashes: Vec<RpcHash>,
    pub accepted_transaction_ids: Vec<RpcAcceptedTransactionIds>,
}

impl GetVirtualChainFromBlockResponse {
    pub fn new(
        removed_chain_block_hashes: Vec<RpcHash>,
        added_chain_block_hashes: Vec<RpcHash>,
        accepted_transaction_ids: Vec<RpcAcceptedTransactionIds>,
    ) -> Self {
        Self { removed_chain_block_hashes, added_chain_block_hashes, accepted_transaction_ids }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlocksRequest {
    pub low_hash: Option<RpcHash>,
    pub include_blocks: bool,
    pub include_transactions: bool,
}

impl GetBlocksRequest {
    pub fn new(low_hash: Option<RpcHash>, include_blocks: bool, include_transactions: bool) -> Self {
        Self { low_hash, include_blocks, include_transactions }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetBlocksResponse {
    #[pyo3(get)]
    pub block_hashes: Vec<RpcHash>,
    #[pyo3(get)]
    pub blocks: Vec<RpcBlock>,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlocksResponse {
    pub block_hashes: Vec<RpcHash>,
    pub blocks: Vec<RpcBlock>,
}

impl GetBlocksResponse {
    pub fn new(block_hashes: Vec<RpcHash>, blocks: Vec<RpcBlock>) -> Self {
        Self { block_hashes, blocks }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockCountRequest {}

pub type GetBlockCountResponse = BlockCount;

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockDagInfoRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetBlockDagInfoResponse {
    #[pyo3(get)]
    pub network: RpcNetworkId,
    #[pyo3(get)]
    pub block_count: u64,
    #[pyo3(get)]
    pub header_count: u64,
    #[pyo3(get)]
    pub tip_hashes: Vec<RpcHash>,
    #[pyo3(get)]
    pub difficulty: f64,
    #[pyo3(get)]
    pub past_median_time: u64, // NOTE: i64 in gRPC protowire
    #[pyo3(get)]
    pub virtual_parent_hashes: Vec<RpcHash>,
    #[pyo3(get)]
    pub pruning_point_hash: RpcHash,
    #[pyo3(get)]
    pub virtual_daa_score: u64,
    #[pyo3(get)]
    pub sink: RpcHash,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBlockDagInfoResponse {
    pub network: RpcNetworkId,
    pub block_count: u64,
    pub header_count: u64,
    pub tip_hashes: Vec<RpcHash>,
    pub difficulty: f64,
    pub past_median_time: u64, // NOTE: i64 in gRPC protowire
    pub virtual_parent_hashes: Vec<RpcHash>,
    pub pruning_point_hash: RpcHash,
    pub virtual_daa_score: u64,
    pub sink: RpcHash,
}

impl GetBlockDagInfoResponse {
    pub fn new(
        network: RpcNetworkId,
        block_count: u64,
        header_count: u64,
        tip_hashes: Vec<RpcHash>,
        difficulty: f64,
        past_median_time: u64,
        virtual_parent_hashes: Vec<RpcHash>,
        pruning_point_hash: RpcHash,
        virtual_daa_score: u64,
        sink: RpcHash,
    ) -> Self {
        Self {
            network,
            block_count,
            header_count,
            tip_hashes,
            difficulty,
            past_median_time,
            virtual_parent_hashes,
            pruning_point_hash,
            virtual_daa_score,
            sink,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveFinalityConflictRequest {
    pub finality_block_hash: RpcHash,
}

impl ResolveFinalityConflictRequest {
    pub fn new(finality_block_hash: RpcHash) -> Self {
        Self { finality_block_hash }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveFinalityConflictResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShutdownResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHeadersRequest {
    pub start_hash: RpcHash,
    pub limit: u64,
    pub is_ascending: bool,
}

impl GetHeadersRequest {
    pub fn new(start_hash: RpcHash, limit: u64, is_ascending: bool) -> Self {
        Self { start_hash, limit, is_ascending }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHeadersResponse {
    pub headers: Vec<RpcHeader>,
}

impl GetHeadersResponse {
    pub fn new(headers: Vec<RpcHeader>) -> Self {
        Self { headers }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalanceByAddressRequest {
    pub address: RpcAddress,
}

impl GetBalanceByAddressRequest {
    pub fn new(address: RpcAddress) -> Self {
        Self { address }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalanceByAddressResponse {
    pub balance: u64,
}

impl GetBalanceByAddressResponse {
    pub fn new(balance: u64) -> Self {
        Self { balance }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesByAddressesRequest {
    pub addresses: Vec<RpcAddress>,
}

impl GetBalancesByAddressesRequest {
    pub fn new(addresses: Vec<RpcAddress>) -> Self {
        Self { addresses }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBalancesByAddressesResponse {
    pub entries: Vec<RpcBalancesByAddressesEntry>,
}

impl GetBalancesByAddressesResponse {
    pub fn new(entries: Vec<RpcBalancesByAddressesEntry>) -> Self {
        Self { entries }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSinkBlueScoreRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSinkBlueScoreResponse {
    pub blue_score: u64,
}

impl GetSinkBlueScoreResponse {
    pub fn new(blue_score: u64) -> Self {
        Self { blue_score }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxosByAddressesRequest {
    pub addresses: Vec<RpcAddress>,
}

impl GetUtxosByAddressesRequest {
    pub fn new(addresses: Vec<RpcAddress>) -> Self {
        Self { addresses }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetUtxosByAddressesResponse {
    pub entries: Vec<RpcUtxosByAddressesEntry>,
}

impl GetUtxosByAddressesResponse {
    pub fn new(entries: Vec<RpcUtxosByAddressesEntry>) -> Self {
        Self { entries }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct BanRequest {
    pub ip: RpcIpAddress,
}

impl BanRequest {
    pub fn new(ip: RpcIpAddress) -> Self {
        Self { ip }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct BanResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnbanRequest {
    pub ip: RpcIpAddress,
}

impl UnbanRequest {
    pub fn new(ip: RpcIpAddress) -> Self {
        Self { ip }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnbanResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateNetworkHashesPerSecondRequest {
    pub window_size: u32,
    pub start_hash: Option<RpcHash>,
}

impl EstimateNetworkHashesPerSecondRequest {
    pub fn new(window_size: u32, start_hash: Option<RpcHash>) -> Self {
        Self { window_size, start_hash }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateNetworkHashesPerSecondResponse {
    pub network_hashes_per_second: u64,
}

impl EstimateNetworkHashesPerSecondResponse {
    pub fn new(network_hashes_per_second: u64) -> Self {
        Self { network_hashes_per_second }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMempoolEntriesByAddressesRequest {
    pub addresses: Vec<RpcAddress>,
    pub include_orphan_pool: bool,
    // TODO: replace with `include_transaction_pool`
    pub filter_transaction_pool: bool,
}

impl GetMempoolEntriesByAddressesRequest {
    pub fn new(addresses: Vec<RpcAddress>, include_orphan_pool: bool, filter_transaction_pool: bool) -> Self {
        Self { addresses, include_orphan_pool, filter_transaction_pool }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMempoolEntriesByAddressesResponse {
    pub entries: Vec<RpcMempoolEntryByAddress>,
}

impl GetMempoolEntriesByAddressesResponse {
    pub fn new(entries: Vec<RpcMempoolEntryByAddress>) -> Self {
        Self { entries }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCoinSupplyRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetCoinSupplyResponse {
    #[pyo3(get)]
    pub max_sompi: u64,
    #[pyo3(get)]
    pub circulating_sompi: u64,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCoinSupplyResponse {
    pub max_sompi: u64,
    pub circulating_sompi: u64,
}

impl GetCoinSupplyResponse {
    pub fn new(max_sompi: u64, circulating_sompi: u64) -> Self {
        Self { max_sompi, circulating_sompi }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingRequest {}

impl Serializer for PingRequest {
  fn serialize<W: std::io::Write>(&self, _writer: &mut W) -> std::io::Result<()> {
      Ok(())
  }
}

impl Deserializer for PingRequest {
  fn deserialize<R: std::io::Read>(_reader: &mut R) -> std::io::Result<Self> {
      Ok(Self {})
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingResponse {}

impl Serializer for PingResponse {
  fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
      store!(u8, &1, writer)?;
      Ok(())
  }
}

impl Deserializer for PingResponse {
  fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
      let _version = load!(u8, reader)?;
      Ok(Self {})
  }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionsProfileData {
    pub cpu_usage: f32,
    pub memory_usage: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConnectionsRequest {
    pub include_profile_data: bool,
}

impl Serializer for GetConnectionsRequest {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        store!(u8, &1, writer)?;
        store!(bool, &self.include_profile_data, writer)?;
        Ok(())
    }
}

impl Deserializer for GetConnectionsRequest {
    fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let _version = load!(u8, reader)?;
        let include_profile_data = load!(bool, reader)?;
        Ok(Self { include_profile_data })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetConnectionsResponse {
    pub clients: u32,
    pub peers: u16,
    pub profile_data: Option<ConnectionsProfileData>,
}

impl Serializer for GetConnectionsResponse {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        store!(u16, &1, writer)?;
        store!(u32, &self.clients, writer)?;
        store!(u16, &self.peers, writer)?;
        store!(Option<ConnectionsProfileData>, &self.profile_data, writer)?;
        Ok(())
    }
}

impl Deserializer for GetConnectionsResponse {
    fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let _version = load!(u16, reader)?;
        let clients = load!(u32, reader)?;
        let peers = load!(u16, reader)?;
        let extra = load!(Option<ConnectionsProfileData>, reader)?;
        Ok(Self { clients, peers, profile_data: extra })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSystemInfoRequest {}

impl Serializer for GetSystemInfoRequest {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        store!(u16, &1, writer)?;

        Ok(())
    }
}

impl Deserializer for GetSystemInfoRequest {
    fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let _version = load!(u16, reader)?;

        Ok(Self {})
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSystemInfoResponse {
    pub version: String,
    pub system_id: Option<Vec<u8>>,
    pub git_hash: Option<Vec<u8>>,
    pub cpu_physical_cores: u16,
    pub total_memory: u64,
    pub fd_limit: u32,
    pub proxy_socket_limit_per_cpu_core: Option<u32>,
}

impl std::fmt::Debug for GetSystemInfoResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GetSystemInfoResponse")
            .field("version", &self.version)
            .field("system_id", &self.system_id.as_ref().map(|id| id.to_hex()))
            .field("git_hash", &self.git_hash.as_ref().map(|hash| hash.to_hex()))
            .field("cpu_physical_cores", &self.cpu_physical_cores)
            .field("total_memory", &self.total_memory)
            .field("fd_limit", &self.fd_limit)
            .field("proxy_socket_limit_per_cpu_core", &self.proxy_socket_limit_per_cpu_core)
            .finish()
    }
}

impl Serializer for GetSystemInfoResponse {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        store!(u16, &2, writer)?;
        store!(String, &self.version, writer)?;
        store!(Option<Vec<u8>>, &self.system_id, writer)?;
        store!(Option<Vec<u8>>, &self.git_hash, writer)?;
        store!(u16, &self.cpu_physical_cores, writer)?;
        store!(u64, &self.total_memory, writer)?;
        store!(u32, &self.fd_limit, writer)?;
        store!(Option<u32>, &self.proxy_socket_limit_per_cpu_core, writer)?;

        Ok(())
    }
}

impl Deserializer for GetSystemInfoResponse {
    fn deserialize<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let payload_version = load!(u16, reader)?;
        let version = load!(String, reader)?;
        let system_id = load!(Option<Vec<u8>>, reader)?;
        let git_hash = load!(Option<Vec<u8>>, reader)?;
        let cpu_physical_cores = load!(u16, reader)?;
        let total_memory = load!(u64, reader)?;
        let fd_limit = load!(u32, reader)?;

        let proxy_socket_limit_per_cpu_core = if payload_version > 1 { load!(Option<u32>, reader)? } else { None };

        Ok(Self { version, system_id, git_hash, cpu_physical_cores, total_memory, fd_limit, proxy_socket_limit_per_cpu_core })
    }
}

// TODO - custom wRPC commands (need review and implementation in gRPC)

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMetricsRequest {
    pub process_metrics: bool,
    pub connection_metrics: bool,
    pub bandwidth_metrics: bool,
    pub consensus_metrics: bool,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct ProcessMetrics {
    #[pyo3(get)]
    pub resident_set_size: u64,
    #[pyo3(get)]
    pub virtual_memory_size: u64,
    #[pyo3(get)]
    pub core_num: u32,
    #[pyo3(get)]
    pub cpu_usage: f32,
    #[pyo3(get)]
    pub fd_num: u32,
    #[pyo3(get)]
    pub disk_io_read_bytes: u64,
    #[pyo3(get)]
    pub disk_io_write_bytes: u64,
    #[pyo3(get)]
    pub disk_io_read_per_sec: f32,
    #[pyo3(get)]
    pub disk_io_write_per_sec: f32,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessMetrics {
    pub resident_set_size: u64,
    pub virtual_memory_size: u64,
    pub core_num: u32,
    pub cpu_usage: f32,
    pub fd_num: u32,
    pub disk_io_read_bytes: u64,
    pub disk_io_write_bytes: u64,
    pub disk_io_read_per_sec: f32,
    pub disk_io_write_per_sec: f32,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct ConnectionMetrics {
    #[pyo3(get)]
    pub borsh_live_connections: u32,
    #[pyo3(get)]
    pub borsh_connection_attempts: u64,
    #[pyo3(get)]
    pub borsh_handshake_failures: u64,
    #[pyo3(get)]
    pub json_live_connections: u32,
    #[pyo3(get)]
    pub json_connection_attempts: u64,
    #[pyo3(get)]
    pub json_handshake_failures: u64,

    #[pyo3(get)]
    pub active_peers: u32,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionMetrics {
    pub borsh_live_connections: u32,
    pub borsh_connection_attempts: u64,
    pub borsh_handshake_failures: u64,
    pub json_live_connections: u32,
    pub json_connection_attempts: u64,
    pub json_handshake_failures: u64,

    pub active_peers: u32,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct BandwidthMetrics {
    #[pyo3(get)]
    pub borsh_bytes_tx: u64,
    #[pyo3(get)]
    pub borsh_bytes_rx: u64,
    #[pyo3(get)]
    pub json_bytes_tx: u64,
    #[pyo3(get)]
    pub json_bytes_rx: u64,
    #[pyo3(get)]
    pub p2p_bytes_tx: u64,
    #[pyo3(get)]
    pub p2p_bytes_rx: u64,
    #[pyo3(get)]
    pub grpc_bytes_tx: u64,
    #[pyo3(get)]
    pub grpc_bytes_rx: u64,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct BandwidthMetrics {
    pub borsh_bytes_tx: u64,
    pub borsh_bytes_rx: u64,
    pub json_bytes_tx: u64,
    pub json_bytes_rx: u64,
    pub p2p_bytes_tx: u64,
    pub p2p_bytes_rx: u64,
    pub grpc_bytes_tx: u64,
    pub grpc_bytes_rx: u64,
}

#[derive(Default, Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct ConsensusMetrics {
    #[pyo3(get)]
    pub node_blocks_submitted_count: u64,
    #[pyo3(get)]
    pub node_headers_processed_count: u64,
    #[pyo3(get)]
    pub node_dependencies_processed_count: u64,
    #[pyo3(get)]
    pub node_bodies_processed_count: u64,
    #[pyo3(get)]
    pub node_transactions_processed_count: u64,
    #[pyo3(get)]
    pub node_chain_blocks_processed_count: u64,
    #[pyo3(get)]
    pub node_mass_processed_count: u64,

    #[pyo3(get)]
    pub node_database_blocks_count: u64,
    #[pyo3(get)]
    pub node_database_headers_count: u64,

    #[pyo3(get)]
    pub network_mempool_size: u64,
    #[pyo3(get)]
    pub network_tip_hashes_count: u32,
    #[pyo3(get)]
    pub network_difficulty: f64,
    #[pyo3(get)]
    pub network_past_median_time: u64,
    #[pyo3(get)]
    pub network_virtual_parent_hashes_count: u32,
    #[pyo3(get)]
    pub network_virtual_daa_score: u64,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConsensusMetrics {
    pub node_blocks_submitted_count: u64,
    pub node_headers_processed_count: u64,
    pub node_dependencies_processed_count: u64,
    pub node_bodies_processed_count: u64,
    pub node_transactions_processed_count: u64,
    pub node_chain_blocks_processed_count: u64,
    pub node_mass_processed_count: u64,

    pub node_database_blocks_count: u64,
    pub node_database_headers_count: u64,

    pub network_mempool_size: u64,
    pub network_tip_hashes_count: u32,
    pub network_difficulty: f64,
    pub network_past_median_time: u64,
    pub network_virtual_parent_hashes_count: u32,
    pub network_virtual_daa_score: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetMetricsResponse {
    #[pyo3(get)]
    pub server_time: u64,
    #[pyo3(get)]
    pub process_metrics: Option<ProcessMetrics>,
    #[pyo3(get)]
    pub connection_metrics: Option<ConnectionMetrics>,
    #[pyo3(get)]
    pub bandwidth_metrics: Option<BandwidthMetrics>,
    #[pyo3(get)]
    pub consensus_metrics: Option<ConsensusMetrics>,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMetricsResponse {
    pub server_time: u64,
    pub process_metrics: Option<ProcessMetrics>,
    pub connection_metrics: Option<ConnectionMetrics>,
    pub bandwidth_metrics: Option<BandwidthMetrics>,
    pub consensus_metrics: Option<ConsensusMetrics>,
}

impl GetMetricsResponse {
    pub fn new(
        server_time: u64,
        process_metrics: Option<ProcessMetrics>,
        connection_metrics: Option<ConnectionMetrics>,
        bandwidth_metrics: Option<BandwidthMetrics>,
        consensus_metrics: Option<ConsensusMetrics>,
    ) -> Self {
        Self { process_metrics, connection_metrics, bandwidth_metrics, consensus_metrics, server_time }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetServerInfoRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
#[cfg(not(target_family = "wasm"))]
#[pyclass]
pub struct GetServerInfoResponse {
    #[pyo3(get)]
    pub rpc_api_version: [u16; 4],
    #[pyo3(get)]
    pub server_version: String,
    pub network_id: RpcNetworkId,
    #[pyo3(get)]
    pub has_utxo_index: bool,
    #[pyo3(get)]
    pub is_synced: bool,
    #[pyo3(get)]
    pub virtual_daa_score: u64,
}

#[cfg(target_family = "wasm")]
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetServerInfoResponse {
    pub rpc_api_version: [u16; 4],
    pub server_version: String,
    pub network_id: RpcNetworkId,
    pub has_utxo_index: bool,
    pub is_synced: bool,
    pub virtual_daa_score: u64,
}

#[cfg(not(target_family = "wasm"))]
#[pymethods]
impl GetServerInfoResponse {
    #[getter]
    pub fn network_id(&self) -> String {
        self.network_id.to_string()
    }
}

#[cfg(target_family = "wasm")]
impl GetServerInfoResponse {
    pub fn network_id(&self) -> String {
        self.network_id.to_string()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSyncStatusRequest {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSyncStatusResponse {
    pub is_synced: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDaaScoreTimestampEstimateRequest {
    pub daa_scores: Vec<u64>,
}

impl GetDaaScoreTimestampEstimateRequest {
    pub fn new(daa_scores: Vec<u64>) -> Self {
        Self { daa_scores }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDaaScoreTimestampEstimateResponse {
    pub timestamps: Vec<u64>,
}

impl GetDaaScoreTimestampEstimateResponse {
    pub fn new(timestamps: Vec<u64>) -> Self {
        Self { timestamps }
    }
}

// ----------------------------------------------------------------------------
// Subscriptions & notifications
// ----------------------------------------------------------------------------

// ~~~~~~~~~~~~~~~~~~~~~~
// BlockAddedNotification

/// NotifyBlockAddedRequest registers this connection for blockAdded notifications.
///
/// See: BlockAddedNotification
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyBlockAddedRequest {
    pub command: Command,
}
impl NotifyBlockAddedRequest {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyBlockAddedResponse {}

/// BlockAddedNotification is sent whenever a blocks has been added (NOT accepted)
/// into the DAG.
///
/// See: NotifyBlockAddedRequest
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockAddedNotification {
    pub block: Arc<RpcBlock>,
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// VirtualChainChangedNotification

// NotifyVirtualChainChangedRequest registers this connection for
// virtualDaaScoreChanged notifications.
//
// See: VirtualChainChangedNotification
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyVirtualChainChangedRequest {
    pub include_accepted_transaction_ids: bool,
    pub command: Command,
}

impl NotifyVirtualChainChangedRequest {
    pub fn new(include_accepted_transaction_ids: bool, command: Command) -> Self {
        Self { include_accepted_transaction_ids, command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyVirtualChainChangedResponse {}

// VirtualChainChangedNotification is sent whenever the DAG's selected parent
// chain had changed.
//
// See: NotifyVirtualChainChangedRequest
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualChainChangedNotification {
    pub removed_chain_block_hashes: Arc<Vec<RpcHash>>,
    pub added_chain_block_hashes: Arc<Vec<RpcHash>>,
    pub accepted_transaction_ids: Arc<Vec<RpcAcceptedTransactionIds>>,
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// FinalityConflictNotification

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyFinalityConflictRequest {
    pub command: Command,
}

impl NotifyFinalityConflictRequest {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyFinalityConflictResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalityConflictNotification {
    pub violating_block_hash: RpcHash,
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// FinalityConflictResolvedNotification

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyFinalityConflictResolvedRequest {
    pub command: Command,
}

impl NotifyFinalityConflictResolvedRequest {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyFinalityConflictResolvedResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalityConflictResolvedNotification {
    pub finality_block_hash: RpcHash,
}

// ~~~~~~~~~~~~~~~~~~~~~~~~
// UtxosChangedNotification

// NotifyUtxosChangedRequestMessage registers this connection for utxoChanged notifications
// for the given addresses. Depending on the provided `command`, notifications will
// start or stop for the provided `addresses`.
//
// If `addresses` is empty, the notifications will start or stop for all addresses.
//
// This call is only available when this waglaylad was started with `--utxoindex`
//
// See: UtxosChangedNotification
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyUtxosChangedRequest {
    pub addresses: Vec<RpcAddress>,
    pub command: Command,
}

impl NotifyUtxosChangedRequest {
    pub fn new(addresses: Vec<RpcAddress>, command: Command) -> Self {
        Self { addresses, command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyUtxosChangedResponse {}

// UtxosChangedNotificationMessage is sent whenever the UTXO index had been updated.
//
// See: NotifyUtxosChangedRequest
#[derive(Clone, Debug, Default, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtxosChangedNotification {
    pub added: Arc<Vec<RpcUtxosByAddressesEntry>>,
    pub removed: Arc<Vec<RpcUtxosByAddressesEntry>>,
}

impl UtxosChangedNotification {
    pub(crate) fn apply_utxos_changed_subscription(
        &self,
        subscription: &UtxosChangedSubscription,
        context: &SubscriptionContext,
    ) -> Option<Self> {
        if subscription.to_all() {
            Some(self.clone())
        } else {
            let added = Self::filter_utxos(&self.added, subscription, context);
            let removed = Self::filter_utxos(&self.removed, subscription, context);
            if added.is_empty() && removed.is_empty() {
                None
            } else {
                debug!("CRPC, Creating UtxosChanged notifications with {} added and {} removed utxos", added.len(), removed.len());
                Some(Self { added: Arc::new(added), removed: Arc::new(removed) })
            }
        }
    }

    fn filter_utxos(
        utxo_set: &[RpcUtxosByAddressesEntry],
        subscription: &UtxosChangedSubscription,
        context: &SubscriptionContext,
    ) -> Vec<RpcUtxosByAddressesEntry> {
        let subscription_data = subscription.data();
        utxo_set.iter().filter(|x| subscription_data.contains(&x.utxo_entry.script_public_key, context)).cloned().collect()
    }
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// SinkBlueScoreChangedNotification

// NotifySinkBlueScoreChangedRequest registers this connection for
// sinkBlueScoreChanged notifications.
//
// See: SinkBlueScoreChangedNotification
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifySinkBlueScoreChangedRequest {
    pub command: Command,
}

impl NotifySinkBlueScoreChangedRequest {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifySinkBlueScoreChangedResponse {}

// SinkBlueScoreChangedNotification is sent whenever the blue score
// of the virtual's selected parent changes.
//
/// See: NotifySinkBlueScoreChangedRequest
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct SinkBlueScoreChangedNotification {
    pub sink_blue_score: u64,
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// VirtualDaaScoreChangedNotification

// NotifyVirtualDaaScoreChangedRequest registers this connection for
// virtualDaaScoreChanged notifications.
//
// See: VirtualDaaScoreChangedNotification
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyVirtualDaaScoreChangedRequest {
    pub command: Command,
}

impl NotifyVirtualDaaScoreChangedRequest {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyVirtualDaaScoreChangedResponse {}

// VirtualDaaScoreChangedNotification is sent whenever the DAA score
// of the virtual changes.
//
// See NotifyVirtualDaaScoreChangedRequest
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct VirtualDaaScoreChangedNotification {
    pub virtual_daa_score: u64,
}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// PruningPointUtxoSetOverrideNotification

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyPruningPointUtxoSetOverrideRequest {
    pub command: Command,
}

impl NotifyPruningPointUtxoSetOverrideRequest {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyPruningPointUtxoSetOverrideResponse {}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct PruningPointUtxoSetOverrideNotification {}

// ~~~~~~~~~~~~~~~~~~~~~~~~~~~~
// NewBlockTemplateNotification

/// NotifyNewBlockTemplateRequest registers this connection for blockAdded notifications.
///
/// See: NewBlockTemplateNotification
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyNewBlockTemplateRequest {
    pub command: Command,
}
impl NotifyNewBlockTemplateRequest {
    pub fn new(command: Command) -> Self {
        Self { command }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotifyNewBlockTemplateResponse {}

/// NewBlockTemplateNotification is sent whenever a blocks has been added (NOT accepted)
/// into the DAG.
///
/// See: NotifyNewBlockTemplateRequest
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewBlockTemplateNotification {}

///
///  wRPC response for RpcApiOps::Subscribe request
///
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscribeResponse {
    id: u64,
}

impl SubscribeResponse {
    pub fn new(id: u64) -> Self {
        Self { id }
    }
}

///
///  wRPC response for RpcApiOps::Unsubscribe request
///
#[derive(Clone, Debug, Serialize, Deserialize, BorshSerialize, BorshDeserialize)]
#[serde(rename_all = "camelCase")]
pub struct UnsubscribeResponse {}
