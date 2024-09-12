use waglayla_grpc_core::protowire::{waglaylad_request, waglaylad_response, WaglayladRequest, WaglayladResponse};

pub(crate) trait Matcher<T> {
    fn is_matching(&self, response: T) -> bool;
}

impl Matcher<&waglaylad_response::Payload> for waglaylad_request::Payload {
    fn is_matching(&self, response: &waglaylad_response::Payload) -> bool {
        use waglaylad_request::Payload;
        match self {
            // TODO: implement for each payload variant supporting request/response pairing
            Payload::GetBlockRequest(ref request) => {
                if let waglaylad_response::Payload::GetBlockResponse(ref response) = response {
                    if let Some(block) = response.block.as_ref() {
                        if let Some(verbose_data) = block.verbose_data.as_ref() {
                            return verbose_data.hash == request.hash;
                        }
                        return true;
                    } else if let Some(error) = response.error.as_ref() {
                        // the response error message should contain the requested hash
                        return error.message.contains(request.hash.as_str());
                    }
                }
                false
            }

            _ => true,
        }
    }
}

impl Matcher<&WaglayladResponse> for WaglayladRequest {
    fn is_matching(&self, response: &WaglayladResponse) -> bool {
        if let Some(ref response) = response.payload {
            if let Some(ref request) = self.payload {
                return request.is_matching(response);
            }
        }
        false
    }
}
