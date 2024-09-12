use crate::v5::{
    address::{ReceiveAddressesFlow, SendAddressesFlow},
    blockrelay::{flow::HandleRelayInvsFlow, handle_requests::HandleRelayBlockRequests},
    ibd::IbdFlow,
    ping::{ReceivePingsFlow, SendPingsFlow},
    request_antipast::HandleAntipastRequests,
    request_block_locator::RequestBlockLocatorFlow,
    request_headers::RequestHeadersFlow,
    request_ibd_blocks::HandleIbdBlockRequests,
    request_ibd_chain_block_locator::RequestIbdChainBlockLocatorFlow,
    request_pp_proof::RequestPruningPointProofFlow,
    request_pruning_point_utxo_set::RequestPruningPointUtxoSetFlow,
    txrelay::flow::{RelayTransactionsFlow, RequestTransactionsFlow},
};
use crate::{flow_context::FlowContext, flow_trait::Flow};

use waglayla_p2p_lib::{WaglayladMessagePayloadType, Router, SharedIncomingRoute};
use waglayla_utils::channel;
use std::sync::Arc;

use crate::v6::request_pruning_point_and_anticone::PruningPointAndItsAnticoneRequestsFlow;

pub(crate) mod request_pruning_point_and_anticone;

pub fn register(ctx: FlowContext, router: Arc<Router>) -> Vec<Box<dyn Flow>> {
    // IBD flow <-> invs flow communication uses a job channel in order to always
    // maintain at most a single pending job which can be updated
    let (ibd_sender, relay_receiver) = channel::job();

    let mut flows: Vec<Box<dyn Flow>> = vec![
        Box::new(IbdFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![
                WaglayladMessagePayloadType::BlockHeaders,
                WaglayladMessagePayloadType::DoneHeaders,
                WaglayladMessagePayloadType::IbdBlockLocatorHighestHash,
                WaglayladMessagePayloadType::IbdBlockLocatorHighestHashNotFound,
                WaglayladMessagePayloadType::BlockWithTrustedDataV4,
                WaglayladMessagePayloadType::DoneBlocksWithTrustedData,
                WaglayladMessagePayloadType::IbdChainBlockLocator,
                WaglayladMessagePayloadType::IbdBlock,
                WaglayladMessagePayloadType::TrustedData,
                WaglayladMessagePayloadType::PruningPoints,
                WaglayladMessagePayloadType::PruningPointProof,
                WaglayladMessagePayloadType::UnexpectedPruningPoint,
                WaglayladMessagePayloadType::PruningPointUtxoSetChunk,
                WaglayladMessagePayloadType::DonePruningPointUtxoSetChunks,
            ]),
            relay_receiver,
        )),
        Box::new(HandleRelayBlockRequests::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestRelayBlocks]),
        )),
        Box::new(ReceivePingsFlow::new(ctx.clone(), router.clone(), router.subscribe(vec![WaglayladMessagePayloadType::Ping]))),
        Box::new(SendPingsFlow::new(ctx.clone(), router.clone(), router.subscribe(vec![WaglayladMessagePayloadType::Pong]))),
        Box::new(RequestHeadersFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestHeaders, WaglayladMessagePayloadType::RequestNextHeaders]),
        )),
        Box::new(RequestPruningPointProofFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestPruningPointProof]),
        )),
        Box::new(RequestIbdChainBlockLocatorFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestIbdChainBlockLocator]),
        )),
        Box::new(PruningPointAndItsAnticoneRequestsFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![
                WaglayladMessagePayloadType::RequestPruningPointAndItsAnticone,
                WaglayladMessagePayloadType::RequestNextPruningPointAndItsAnticoneBlocks,
            ]),
        )),
        Box::new(RequestPruningPointUtxoSetFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![
                WaglayladMessagePayloadType::RequestPruningPointUtxoSet,
                WaglayladMessagePayloadType::RequestNextPruningPointUtxoSetChunk,
            ]),
        )),
        Box::new(HandleIbdBlockRequests::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestIbdBlocks]),
        )),
        Box::new(HandleAntipastRequests::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestAntipast]),
        )),
        Box::new(RelayTransactionsFlow::new(
            ctx.clone(),
            router.clone(),
            router
                .subscribe_with_capacity(vec![WaglayladMessagePayloadType::InvTransactions], RelayTransactionsFlow::invs_channel_size()),
            router.subscribe_with_capacity(
                vec![WaglayladMessagePayloadType::Transaction, WaglayladMessagePayloadType::TransactionNotFound],
                RelayTransactionsFlow::txs_channel_size(),
            ),
        )),
        Box::new(RequestTransactionsFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestTransactions]),
        )),
        Box::new(ReceiveAddressesFlow::new(ctx.clone(), router.clone(), router.subscribe(vec![WaglayladMessagePayloadType::Addresses]))),
        Box::new(SendAddressesFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestAddresses]),
        )),
        Box::new(RequestBlockLocatorFlow::new(
            ctx.clone(),
            router.clone(),
            router.subscribe(vec![WaglayladMessagePayloadType::RequestBlockLocator]),
        )),
    ];

    let invs_route = router.subscribe_with_capacity(vec![WaglayladMessagePayloadType::InvRelayBlock], ctx.block_invs_channel_size());
    let shared_invs_route = SharedIncomingRoute::new(invs_route);

    let num_relay_flows = (ctx.config.bps() as usize / 2).max(1);
    flows.extend((0..num_relay_flows).map(|_| {
        Box::new(HandleRelayInvsFlow::new(
            ctx.clone(),
            router.clone(),
            shared_invs_route.clone(),
            router.subscribe(vec![]),
            ibd_sender.clone(),
        )) as Box<dyn Flow>
    }));

    // The reject message is handled as a special case by the router
    // WaglayladMessagePayloadType::Reject,

    // We do not register the below two messages since they are deprecated also in go-waglayla
    // WaglayladMessagePayloadType::BlockWithTrustedData,
    // WaglayladMessagePayloadType::IbdBlockLocator,

    flows
}
