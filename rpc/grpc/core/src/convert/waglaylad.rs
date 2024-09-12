use crate::protowire::{waglaylad_request, WaglayladRequest, WaglayladResponse};

impl From<waglaylad_request::Payload> for WaglayladRequest {
    fn from(item: waglaylad_request::Payload) -> Self {
        WaglayladRequest { id: 0, payload: Some(item) }
    }
}

impl AsRef<WaglayladRequest> for WaglayladRequest {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl AsRef<WaglayladResponse> for WaglayladResponse {
    fn as_ref(&self) -> &Self {
        self
    }
}

pub mod waglaylad_request_convert {
    use crate::protowire::*;
    use waglayla_rpc_core::{RpcError, RpcResult};

    impl_into_waglaylad_request!(Shutdown);
    impl_into_waglaylad_request!(SubmitBlock);
    impl_into_waglaylad_request!(GetBlockTemplate);
    impl_into_waglaylad_request!(GetBlock);
    impl_into_waglaylad_request!(GetInfo);

    impl_into_waglaylad_request!(GetCurrentNetwork);
    impl_into_waglaylad_request!(GetPeerAddresses);
    impl_into_waglaylad_request!(GetSink);
    impl_into_waglaylad_request!(GetMempoolEntry);
    impl_into_waglaylad_request!(GetMempoolEntries);
    impl_into_waglaylad_request!(GetConnectedPeerInfo);
    impl_into_waglaylad_request!(AddPeer);
    impl_into_waglaylad_request!(SubmitTransaction);
    impl_into_waglaylad_request!(GetSubnetwork);
    impl_into_waglaylad_request!(GetVirtualChainFromBlock);
    impl_into_waglaylad_request!(GetBlocks);
    impl_into_waglaylad_request!(GetBlockCount);
    impl_into_waglaylad_request!(GetBlockDagInfo);
    impl_into_waglaylad_request!(ResolveFinalityConflict);
    impl_into_waglaylad_request!(GetHeaders);
    impl_into_waglaylad_request!(GetUtxosByAddresses);
    impl_into_waglaylad_request!(GetBalanceByAddress);
    impl_into_waglaylad_request!(GetBalancesByAddresses);
    impl_into_waglaylad_request!(GetSinkBlueScore);
    impl_into_waglaylad_request!(Ban);
    impl_into_waglaylad_request!(Unban);
    impl_into_waglaylad_request!(EstimateNetworkHashesPerSecond);
    impl_into_waglaylad_request!(GetMempoolEntriesByAddresses);
    impl_into_waglaylad_request!(GetCoinSupply);
    impl_into_waglaylad_request!(Ping);
    impl_into_waglaylad_request!(GetMetrics);
    impl_into_waglaylad_request!(GetServerInfo);
    impl_into_waglaylad_request!(GetSyncStatus);
    impl_into_waglaylad_request!(GetDaaScoreTimestampEstimate);

    impl_into_waglaylad_request!(NotifyBlockAdded);
    impl_into_waglaylad_request!(NotifyNewBlockTemplate);
    impl_into_waglaylad_request!(NotifyUtxosChanged);
    impl_into_waglaylad_request!(NotifyPruningPointUtxoSetOverride);
    impl_into_waglaylad_request!(NotifyFinalityConflict);
    impl_into_waglaylad_request!(NotifyVirtualDaaScoreChanged);
    impl_into_waglaylad_request!(NotifyVirtualChainChanged);
    impl_into_waglaylad_request!(NotifySinkBlueScoreChanged);

    macro_rules! impl_into_waglaylad_request {
        ($name:tt) => {
            paste::paste! {
                impl_into_waglaylad_request_ex!(waglayla_rpc_core::[<$name Request>],[<$name RequestMessage>],[<$name Request>]);
            }
        };
    }

    use impl_into_waglaylad_request;

    macro_rules! impl_into_waglaylad_request_ex {
        // ($($core_struct:ident)::+, $($protowire_struct:ident)::+, $($variant:ident)::+) => {
        ($core_struct:path, $protowire_struct:ident, $variant:ident) => {
            // ----------------------------------------------------------------------------
            // rpc_core to protowire
            // ----------------------------------------------------------------------------

            impl From<&$core_struct> for waglaylad_request::Payload {
                fn from(item: &$core_struct) -> Self {
                    Self::$variant(item.into())
                }
            }

            impl From<&$core_struct> for WaglayladRequest {
                fn from(item: &$core_struct) -> Self {
                    Self { id: 0, payload: Some(item.into()) }
                }
            }

            impl From<$core_struct> for waglaylad_request::Payload {
                fn from(item: $core_struct) -> Self {
                    Self::$variant((&item).into())
                }
            }

            impl From<$core_struct> for WaglayladRequest {
                fn from(item: $core_struct) -> Self {
                    Self { id: 0, payload: Some((&item).into()) }
                }
            }

            // ----------------------------------------------------------------------------
            // protowire to rpc_core
            // ----------------------------------------------------------------------------

            impl TryFrom<&waglaylad_request::Payload> for $core_struct {
                type Error = RpcError;
                fn try_from(item: &waglaylad_request::Payload) -> RpcResult<Self> {
                    if let waglaylad_request::Payload::$variant(request) = item {
                        request.try_into()
                    } else {
                        Err(RpcError::MissingRpcFieldError("Payload".to_string(), stringify!($variant).to_string()))
                    }
                }
            }

            impl TryFrom<&WaglayladRequest> for $core_struct {
                type Error = RpcError;
                fn try_from(item: &WaglayladRequest) -> RpcResult<Self> {
                    item.payload
                        .as_ref()
                        .ok_or(RpcError::MissingRpcFieldError("WaglaylaRequest".to_string(), "Payload".to_string()))?
                        .try_into()
                }
            }

            impl From<$protowire_struct> for WaglayladRequest {
                fn from(item: $protowire_struct) -> Self {
                    Self { id: 0, payload: Some(waglaylad_request::Payload::$variant(item)) }
                }
            }

            impl From<$protowire_struct> for waglaylad_request::Payload {
                fn from(item: $protowire_struct) -> Self {
                    waglaylad_request::Payload::$variant(item)
                }
            }
        };
    }
    use impl_into_waglaylad_request_ex;
}

pub mod waglaylad_response_convert {
    use crate::protowire::*;
    use waglayla_rpc_core::{RpcError, RpcResult};

    impl_into_waglaylad_response!(Shutdown);
    impl_into_waglaylad_response!(SubmitBlock);
    impl_into_waglaylad_response!(GetBlockTemplate);
    impl_into_waglaylad_response!(GetBlock);
    impl_into_waglaylad_response!(GetInfo);
    impl_into_waglaylad_response!(GetCurrentNetwork);

    impl_into_waglaylad_response!(GetPeerAddresses);
    impl_into_waglaylad_response!(GetSink);
    impl_into_waglaylad_response!(GetMempoolEntry);
    impl_into_waglaylad_response!(GetMempoolEntries);
    impl_into_waglaylad_response!(GetConnectedPeerInfo);
    impl_into_waglaylad_response!(AddPeer);
    impl_into_waglaylad_response!(SubmitTransaction);
    impl_into_waglaylad_response!(GetSubnetwork);
    impl_into_waglaylad_response!(GetVirtualChainFromBlock);
    impl_into_waglaylad_response!(GetBlocks);
    impl_into_waglaylad_response!(GetBlockCount);
    impl_into_waglaylad_response!(GetBlockDagInfo);
    impl_into_waglaylad_response!(ResolveFinalityConflict);
    impl_into_waglaylad_response!(GetHeaders);
    impl_into_waglaylad_response!(GetUtxosByAddresses);
    impl_into_waglaylad_response!(GetBalanceByAddress);
    impl_into_waglaylad_response!(GetBalancesByAddresses);
    impl_into_waglaylad_response!(GetSinkBlueScore);
    impl_into_waglaylad_response!(Ban);
    impl_into_waglaylad_response!(Unban);
    impl_into_waglaylad_response!(EstimateNetworkHashesPerSecond);
    impl_into_waglaylad_response!(GetMempoolEntriesByAddresses);
    impl_into_waglaylad_response!(GetCoinSupply);
    impl_into_waglaylad_response!(Ping);
    impl_into_waglaylad_response!(GetMetrics);
    impl_into_waglaylad_response!(GetServerInfo);
    impl_into_waglaylad_response!(GetSyncStatus);
    impl_into_waglaylad_response!(GetDaaScoreTimestampEstimate);

    impl_into_waglaylad_notify_response!(NotifyBlockAdded);
    impl_into_waglaylad_notify_response!(NotifyNewBlockTemplate);
    impl_into_waglaylad_notify_response!(NotifyUtxosChanged);
    impl_into_waglaylad_notify_response!(NotifyPruningPointUtxoSetOverride);
    impl_into_waglaylad_notify_response!(NotifyFinalityConflict);
    impl_into_waglaylad_notify_response!(NotifyVirtualDaaScoreChanged);
    impl_into_waglaylad_notify_response!(NotifyVirtualChainChanged);
    impl_into_waglaylad_notify_response!(NotifySinkBlueScoreChanged);

    impl_into_waglaylad_notify_response!(NotifyUtxosChanged, StopNotifyingUtxosChanged);
    impl_into_waglaylad_notify_response!(NotifyPruningPointUtxoSetOverride, StopNotifyingPruningPointUtxoSetOverride);

    macro_rules! impl_into_waglaylad_response {
        ($name:tt) => {
            paste::paste! {
                impl_into_waglaylad_response_ex!(waglayla_rpc_core::[<$name Response>],[<$name ResponseMessage>],[<$name Response>]);
            }
        };
        ($core_name:tt, $protowire_name:tt) => {
            paste::paste! {
                impl_into_waglaylad_response_base!(waglayla_rpc_core::[<$core_name Response>],[<$protowire_name ResponseMessage>],[<$protowire_name Response>]);
            }
        };
    }
    use impl_into_waglaylad_response;

    macro_rules! impl_into_waglaylad_response_base {
        ($core_struct:path, $protowire_struct:ident, $variant:ident) => {
            // ----------------------------------------------------------------------------
            // rpc_core to protowire
            // ----------------------------------------------------------------------------

            impl From<RpcResult<$core_struct>> for $protowire_struct {
                fn from(item: RpcResult<$core_struct>) -> Self {
                    item.as_ref().map_err(|x| (*x).clone()).into()
                }
            }

            impl From<RpcError> for $protowire_struct {
                fn from(item: RpcError) -> Self {
                    let x: RpcResult<&$core_struct> = Err(item);
                    x.into()
                }
            }

            impl From<$protowire_struct> for waglaylad_response::Payload {
                fn from(item: $protowire_struct) -> Self {
                    waglaylad_response::Payload::$variant(item)
                }
            }

            impl From<$protowire_struct> for WaglayladResponse {
                fn from(item: $protowire_struct) -> Self {
                    Self { id: 0, payload: Some(waglaylad_response::Payload::$variant(item)) }
                }
            }
        };
    }
    use impl_into_waglaylad_response_base;

    macro_rules! impl_into_waglaylad_response_ex {
        ($core_struct:path, $protowire_struct:ident, $variant:ident) => {
            // ----------------------------------------------------------------------------
            // rpc_core to protowire
            // ----------------------------------------------------------------------------

            impl From<RpcResult<&$core_struct>> for waglaylad_response::Payload {
                fn from(item: RpcResult<&$core_struct>) -> Self {
                    waglaylad_response::Payload::$variant(item.into())
                }
            }

            impl From<RpcResult<&$core_struct>> for WaglayladResponse {
                fn from(item: RpcResult<&$core_struct>) -> Self {
                    Self { id: 0, payload: Some(item.into()) }
                }
            }

            impl From<RpcResult<$core_struct>> for waglaylad_response::Payload {
                fn from(item: RpcResult<$core_struct>) -> Self {
                    waglaylad_response::Payload::$variant(item.into())
                }
            }

            impl From<RpcResult<$core_struct>> for WaglayladResponse {
                fn from(item: RpcResult<$core_struct>) -> Self {
                    Self { id: 0, payload: Some(item.into()) }
                }
            }

            impl_into_waglaylad_response_base!($core_struct, $protowire_struct, $variant);

            // ----------------------------------------------------------------------------
            // protowire to rpc_core
            // ----------------------------------------------------------------------------

            impl TryFrom<&waglaylad_response::Payload> for $core_struct {
                type Error = RpcError;
                fn try_from(item: &waglaylad_response::Payload) -> RpcResult<Self> {
                    if let waglaylad_response::Payload::$variant(response) = item {
                        response.try_into()
                    } else {
                        Err(RpcError::MissingRpcFieldError("Payload".to_string(), stringify!($variant).to_string()))
                    }
                }
            }

            impl TryFrom<&WaglayladResponse> for $core_struct {
                type Error = RpcError;
                fn try_from(item: &WaglayladResponse) -> RpcResult<Self> {
                    item.payload
                        .as_ref()
                        .ok_or(RpcError::MissingRpcFieldError("WaglaylaResponse".to_string(), "Payload".to_string()))?
                        .try_into()
                }
            }
        };
    }
    use impl_into_waglaylad_response_ex;

    macro_rules! impl_into_waglaylad_notify_response {
        ($name:tt) => {
            impl_into_waglaylad_response!($name);

            paste::paste! {
                impl_into_waglaylad_notify_response_ex!(waglayla_rpc_core::[<$name Response>],[<$name ResponseMessage>]);
            }
        };
        ($core_name:tt, $protowire_name:tt) => {
            impl_into_waglaylad_response!($core_name, $protowire_name);

            paste::paste! {
                impl_into_waglaylad_notify_response_ex!(waglayla_rpc_core::[<$core_name Response>],[<$protowire_name ResponseMessage>]);
            }
        };
    }
    use impl_into_waglaylad_notify_response;

    macro_rules! impl_into_waglaylad_notify_response_ex {
        ($($core_struct:ident)::+, $protowire_struct:ident) => {
            // ----------------------------------------------------------------------------
            // rpc_core to protowire
            // ----------------------------------------------------------------------------

            impl<T> From<Result<(), T>> for $protowire_struct
            where
                T: Into<RpcError>,
            {
                fn from(item: Result<(), T>) -> Self {
                    item
                        .map(|_| $($core_struct)::+{})
                        .map_err(|err| err.into()).into()
                }
            }

        };
    }
    use impl_into_waglaylad_notify_response_ex;
}
