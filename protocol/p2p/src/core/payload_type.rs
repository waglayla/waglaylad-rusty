use crate::pb::waglaylad_message::Payload as WaglayladMessagePayload;

#[repr(u8)]
#[derive(Debug, Copy, Clone, Eq, Hash, PartialEq)]
pub enum WaglayladMessagePayloadType {
    Addresses = 0,
    Block,
    Transaction,
    BlockLocator,
    RequestAddresses,
    RequestRelayBlocks,
    RequestTransactions,
    IbdBlock,
    InvRelayBlock,
    InvTransactions,
    Ping,
    Pong,
    Verack,
    Version,
    TransactionNotFound,
    Reject,
    PruningPointUtxoSetChunk,
    RequestIbdBlocks,
    UnexpectedPruningPoint,
    IbdBlockLocator,
    IbdBlockLocatorHighestHash,
    RequestNextPruningPointUtxoSetChunk,
    DonePruningPointUtxoSetChunks,
    IbdBlockLocatorHighestHashNotFound,
    BlockWithTrustedData,
    DoneBlocksWithTrustedData,
    RequestPruningPointAndItsAnticone,
    BlockHeaders,
    RequestNextHeaders,
    DoneHeaders,
    RequestPruningPointUtxoSet,
    RequestHeaders,
    RequestBlockLocator,
    PruningPoints,
    RequestPruningPointProof,
    PruningPointProof,
    Ready,
    BlockWithTrustedDataV4,
    TrustedData,
    RequestIbdChainBlockLocator,
    IbdChainBlockLocator,
    RequestAntipast,
    RequestNextPruningPointAndItsAnticoneBlocks,
}

impl From<&WaglayladMessagePayload> for WaglayladMessagePayloadType {
    fn from(payload: &WaglayladMessagePayload) -> Self {
        match payload {
            WaglayladMessagePayload::Addresses(_) => WaglayladMessagePayloadType::Addresses,
            WaglayladMessagePayload::Block(_) => WaglayladMessagePayloadType::Block,
            WaglayladMessagePayload::Transaction(_) => WaglayladMessagePayloadType::Transaction,
            WaglayladMessagePayload::BlockLocator(_) => WaglayladMessagePayloadType::BlockLocator,
            WaglayladMessagePayload::RequestAddresses(_) => WaglayladMessagePayloadType::RequestAddresses,
            WaglayladMessagePayload::RequestRelayBlocks(_) => WaglayladMessagePayloadType::RequestRelayBlocks,
            WaglayladMessagePayload::RequestTransactions(_) => WaglayladMessagePayloadType::RequestTransactions,
            WaglayladMessagePayload::IbdBlock(_) => WaglayladMessagePayloadType::IbdBlock,
            WaglayladMessagePayload::InvRelayBlock(_) => WaglayladMessagePayloadType::InvRelayBlock,
            WaglayladMessagePayload::InvTransactions(_) => WaglayladMessagePayloadType::InvTransactions,
            WaglayladMessagePayload::Ping(_) => WaglayladMessagePayloadType::Ping,
            WaglayladMessagePayload::Pong(_) => WaglayladMessagePayloadType::Pong,
            WaglayladMessagePayload::Verack(_) => WaglayladMessagePayloadType::Verack,
            WaglayladMessagePayload::Version(_) => WaglayladMessagePayloadType::Version,
            WaglayladMessagePayload::TransactionNotFound(_) => WaglayladMessagePayloadType::TransactionNotFound,
            WaglayladMessagePayload::Reject(_) => WaglayladMessagePayloadType::Reject,
            WaglayladMessagePayload::PruningPointUtxoSetChunk(_) => WaglayladMessagePayloadType::PruningPointUtxoSetChunk,
            WaglayladMessagePayload::RequestIbdBlocks(_) => WaglayladMessagePayloadType::RequestIbdBlocks,
            WaglayladMessagePayload::UnexpectedPruningPoint(_) => WaglayladMessagePayloadType::UnexpectedPruningPoint,
            WaglayladMessagePayload::IbdBlockLocator(_) => WaglayladMessagePayloadType::IbdBlockLocator,
            WaglayladMessagePayload::IbdBlockLocatorHighestHash(_) => WaglayladMessagePayloadType::IbdBlockLocatorHighestHash,
            WaglayladMessagePayload::RequestNextPruningPointUtxoSetChunk(_) => {
                WaglayladMessagePayloadType::RequestNextPruningPointUtxoSetChunk
            }
            WaglayladMessagePayload::DonePruningPointUtxoSetChunks(_) => WaglayladMessagePayloadType::DonePruningPointUtxoSetChunks,
            WaglayladMessagePayload::IbdBlockLocatorHighestHashNotFound(_) => {
                WaglayladMessagePayloadType::IbdBlockLocatorHighestHashNotFound
            }
            WaglayladMessagePayload::BlockWithTrustedData(_) => WaglayladMessagePayloadType::BlockWithTrustedData,
            WaglayladMessagePayload::DoneBlocksWithTrustedData(_) => WaglayladMessagePayloadType::DoneBlocksWithTrustedData,
            WaglayladMessagePayload::RequestPruningPointAndItsAnticone(_) => WaglayladMessagePayloadType::RequestPruningPointAndItsAnticone,
            WaglayladMessagePayload::BlockHeaders(_) => WaglayladMessagePayloadType::BlockHeaders,
            WaglayladMessagePayload::RequestNextHeaders(_) => WaglayladMessagePayloadType::RequestNextHeaders,
            WaglayladMessagePayload::DoneHeaders(_) => WaglayladMessagePayloadType::DoneHeaders,
            WaglayladMessagePayload::RequestPruningPointUtxoSet(_) => WaglayladMessagePayloadType::RequestPruningPointUtxoSet,
            WaglayladMessagePayload::RequestHeaders(_) => WaglayladMessagePayloadType::RequestHeaders,
            WaglayladMessagePayload::RequestBlockLocator(_) => WaglayladMessagePayloadType::RequestBlockLocator,
            WaglayladMessagePayload::PruningPoints(_) => WaglayladMessagePayloadType::PruningPoints,
            WaglayladMessagePayload::RequestPruningPointProof(_) => WaglayladMessagePayloadType::RequestPruningPointProof,
            WaglayladMessagePayload::PruningPointProof(_) => WaglayladMessagePayloadType::PruningPointProof,
            WaglayladMessagePayload::Ready(_) => WaglayladMessagePayloadType::Ready,
            WaglayladMessagePayload::BlockWithTrustedDataV4(_) => WaglayladMessagePayloadType::BlockWithTrustedDataV4,
            WaglayladMessagePayload::TrustedData(_) => WaglayladMessagePayloadType::TrustedData,
            WaglayladMessagePayload::RequestIbdChainBlockLocator(_) => WaglayladMessagePayloadType::RequestIbdChainBlockLocator,
            WaglayladMessagePayload::IbdChainBlockLocator(_) => WaglayladMessagePayloadType::IbdChainBlockLocator,
            WaglayladMessagePayload::RequestAntipast(_) => WaglayladMessagePayloadType::RequestAntipast,
            WaglayladMessagePayload::RequestNextPruningPointAndItsAnticoneBlocks(_) => {
                WaglayladMessagePayloadType::RequestNextPruningPointAndItsAnticoneBlocks
            }
        }
    }
}
