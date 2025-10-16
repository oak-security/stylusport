mod handler;
mod prompts;
mod resources;
mod server;
mod tools;

use rust_mcp_schema::{
    ClientRequest, JsonrpcError, RpcError, ServerResult,
    schema_utils::{
        ClientMessage, ClientMessages, RequestFromClient, ResultFromServer, ServerJsonrpcResponse,
        ServerMessage,
    },
};

use crate::server::OutputSink;

const MAX_LOG_LINE_BYTES: usize = 1024;

fn sanitize_for_log(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || *c == '\n' || *c == '\t')
        .take(MAX_LOG_LINE_BYTES)
        .collect()
}

fn parse_client_msg(input: &str) -> Option<Vec<ClientMessage>> {
    let Ok(client_msgs) = serde_json::from_str(input) else {
        eprintln!("Unexpected input: {}", sanitize_for_log(input));
        return None;
    };

    let msgs = match client_msgs {
        ClientMessages::Single(msg) => vec![msg],
        ClientMessages::Batch(msgs) => msgs,
    };

    Some(msgs)
}

fn handle_client_msg(msg: ClientMessage, output_sink: &mut OutputSink) {
    let ClientMessage::Request(req_msg) = msg else {
        eprintln!(
            "received non-request: {}",
            sanitize_for_log(&msg.to_string())
        );
        return;
    };

    let request = match req_msg.request {
        RequestFromClient::ClientRequest(client_request) => client_request,
        RequestFromClient::CustomRequest(value) => {
            eprintln!("Unsupported custom request: {value:#}");
            return;
        }
    };

    let result: Result<ServerResult, RpcError> = match request {
        ClientRequest::InitializeRequest(req) => handler::initialize_request(req).map(Into::into),
        ClientRequest::PingRequest(req) => handler::ping_request(req).map(Into::into),
        ClientRequest::ListResourcesRequest(req) => {
            handler::list_resources_request(req).map(Into::into)
        }
        ClientRequest::ListResourceTemplatesRequest(req) => {
            handler::list_resource_templates_request(req).map(Into::into)
        }
        ClientRequest::ReadResourceRequest(req) => {
            handler::read_resource_request(req).map(Into::into)
        }
        ClientRequest::ListPromptsRequest(req) => {
            handler::list_prompts_request(req).map(Into::into)
        }
        ClientRequest::GetPromptRequest(req) => handler::get_prompt_request(req).map(Into::into),
        ClientRequest::ListToolsRequest(req) => handler::list_tools_request(req).map(Into::into),
        ClientRequest::CallToolRequest(req) => handler::call_tool_request(req).map(Into::into),
        ClientRequest::CompleteRequest(_)
        | ClientRequest::SubscribeRequest(_)
        | ClientRequest::UnsubscribeRequest(_)
        | ClientRequest::SetLevelRequest(_) => Err(RpcError::internal_error().with_message(
            format!("missing method handling capability: {}", request.method()),
        )),
    };

    let response = match result {
        Ok(success_res) => ServerMessage::Response(ServerJsonrpcResponse::new(
            req_msg.id,
            ResultFromServer::ServerResult(success_res),
        )),
        Err(rpc_err) => ServerMessage::Error(JsonrpcError::new(rpc_err, req_msg.id)),
    };

    let output = serde_json::to_string(&response).expect("infallible serialization");

    output_sink.send(output);
}

fn main() {
    if let Err(err) = server::start(parse_client_msg, handle_client_msg) {
        eprintln!("Runtime Error: {err}");
    }
}
