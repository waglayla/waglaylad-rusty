use super::method::{DropFn, Method, MethodTrait, RoutingPolicy};
use crate::{
    connection::Connection,
    connection_handler::ServerContext,
    error::{GrpcServerError, GrpcServerResult},
};
use waglayla_grpc_core::{
    ops::WaglayladPayloadOps,
    protowire::{WaglayladRequest, WaglayladResponse},
};
use std::fmt::Debug;
use std::{collections::HashMap, sync::Arc};

pub type WaglayladMethod = Method<ServerContext, Connection, WaglayladRequest, WaglayladResponse>;
pub type DynWaglayladMethod = Arc<dyn MethodTrait<ServerContext, Connection, WaglayladRequest, WaglayladResponse>>;
pub type WaglayladDropFn = DropFn<WaglayladRequest, WaglayladResponse>;
pub type WaglayladRoutingPolicy = RoutingPolicy<WaglayladRequest, WaglayladResponse>;

/// An interface providing methods implementations and a fallback "not implemented" method
/// actually returning a message with a "not implemented" error.
///
/// The interface can provide a method clone for every [`WaglayladPayloadOps`] variant for later
/// processing of related requests.
///
/// It is also possible to directly let the interface itself process a request by invoking
/// the `call()` method.
pub struct Interface {
    server_ctx: ServerContext,
    methods: HashMap<WaglayladPayloadOps, DynWaglayladMethod>,
    method_not_implemented: DynWaglayladMethod,
}

impl Interface {
    pub fn new(server_ctx: ServerContext) -> Self {
        let method_not_implemented = Arc::new(Method::new(|_, _, waglaylad_request: WaglayladRequest| {
            Box::pin(async move {
                match waglaylad_request.payload {
                    Some(ref request) => Ok(WaglayladResponse {
                        id: waglaylad_request.id,
                        payload: Some(WaglayladPayloadOps::from(request).to_error_response(GrpcServerError::MethodNotImplemented.into())),
                    }),
                    None => Err(GrpcServerError::InvalidRequestPayload),
                }
            })
        }));
        Self { server_ctx, methods: Default::default(), method_not_implemented }
    }

    pub fn method(&mut self, op: WaglayladPayloadOps, method: WaglayladMethod) {
        let method: DynWaglayladMethod = Arc::new(method);
        if self.methods.insert(op, method).is_some() {
            panic!("RPC method {op:?} is declared multiple times")
        }
    }

    pub fn replace_method(&mut self, op: WaglayladPayloadOps, method: WaglayladMethod) {
        let method: DynWaglayladMethod = Arc::new(method);
        let _ = self.methods.insert(op, method);
    }

    pub fn set_method_properties(
        &mut self,
        op: WaglayladPayloadOps,
        tasks: usize,
        queue_size: usize,
        routing_policy: WaglayladRoutingPolicy,
    ) {
        self.methods.entry(op).and_modify(|x| {
            let method: Method<ServerContext, Connection, WaglayladRequest, WaglayladResponse> =
                Method::with_properties(x.method_fn(), tasks, queue_size, routing_policy);
            let method: Arc<dyn MethodTrait<ServerContext, Connection, WaglayladRequest, WaglayladResponse>> = Arc::new(method);
            *x = method;
        });
    }

    pub async fn call(
        &self,
        op: &WaglayladPayloadOps,
        connection: Connection,
        request: WaglayladRequest,
    ) -> GrpcServerResult<WaglayladResponse> {
        self.methods.get(op).unwrap_or(&self.method_not_implemented).call(self.server_ctx.clone(), connection, request).await
    }

    pub fn get_method(&self, op: &WaglayladPayloadOps) -> DynWaglayladMethod {
        self.methods.get(op).unwrap_or(&self.method_not_implemented).clone()
    }
}

impl Debug for Interface {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Interface").finish()
    }
}
