use super::error::Result;
use core::fmt::Debug;
use waglayla_grpc_core::{
    ops::WaglayladPayloadOps,
    protowire::{WaglayladRequest, WaglayladResponse},
};
use std::{sync::Arc, time::Duration};
use tokio::sync::oneshot;

pub(crate) mod id;
pub(crate) mod matcher;
pub(crate) mod queue;

pub(crate) trait Resolver: Send + Sync + Debug {
    fn register_request(&self, op: WaglayladPayloadOps, request: &WaglayladRequest) -> WaglayladResponseReceiver;
    fn handle_response(&self, response: WaglayladResponse);
    fn remove_expired_requests(&self, timeout: Duration);
}

pub(crate) type DynResolver = Arc<dyn Resolver>;

pub(crate) type WaglayladResponseSender = oneshot::Sender<Result<WaglayladResponse>>;
pub(crate) type WaglayladResponseReceiver = oneshot::Receiver<Result<WaglayladResponse>>;
