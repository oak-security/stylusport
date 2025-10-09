use rust_mcp_schema::{
    CallToolRequest, CallToolResult, GetPromptRequest, GetPromptResult, Implementation,
    InitializeRequest, InitializeResult, LATEST_PROTOCOL_VERSION, ListPromptsRequest,
    ListPromptsResult, ListResourceTemplatesRequest, ListResourceTemplatesResult,
    ListResourcesRequest, ListResourcesResult, ListToolsRequest, ListToolsResult, PingRequest,
    ReadResourceRequest, ReadResourceResult, Result as CustomResult, RpcError, ServerCapabilities,
    ServerCapabilitiesPrompts, ServerCapabilitiesResources, ServerCapabilitiesTools,
};

pub fn initialize_request(req: InitializeRequest) -> Result<InitializeResult, RpcError> {
    if req.params.protocol_version.as_str() > LATEST_PROTOCOL_VERSION {
        return Err(RpcError::internal_error().with_message(format!(
            "unsupported protocol version, requested: {}, supported: {LATEST_PROTOCOL_VERSION}",
            req.params.protocol_version
        )));
    }

    Ok(InitializeResult {
        capabilities: ServerCapabilities {
            resources: Some(ServerCapabilitiesResources {
                list_changed: Some(false),
                subscribe: Some(false),
            }),
            tools: Some(ServerCapabilitiesTools {
                list_changed: Some(false),
            }),
            prompts: Some(ServerCapabilitiesPrompts {
                list_changed: Some(false),
            }),
            ..Default::default()
        },
        instructions: None,
        meta: None,
        // respond with the requested supported version: https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle#version-negotiation
        protocol_version: req.params.protocol_version,
        server_info: Implementation {
            name: env!("CARGO_PKG_NAME").to_owned(),
            title: Some("StylusPort::Solana MCP Server".to_owned()),
            version: env!("CARGO_PKG_VERSION").to_owned(),
        },
    })
}

// return an empty response: https://modelcontextprotocol.io/specification/2025-06-18/basic/utilities/ping#behavior-requirements
pub fn ping_request(_req: PingRequest) -> Result<CustomResult, RpcError> {
    Ok(CustomResult::default())
}

pub fn list_resources_request(_req: ListResourcesRequest) -> Result<ListResourcesResult, RpcError> {
    Ok(ListResourcesResult {
        meta: None,
        next_cursor: None,
        resources: crate::resources::get_all(),
    })
}

pub fn list_resource_templates_request(
    _req: ListResourceTemplatesRequest,
) -> Result<ListResourceTemplatesResult, RpcError> {
    Ok(ListResourceTemplatesResult {
        meta: None,
        next_cursor: None,
        resource_templates: vec![],
    })
}

pub fn read_resource_request(req: ReadResourceRequest) -> Result<ReadResourceResult, RpcError> {
    let Some(content) = crate::resources::get_resource(&req.params.uri) else {
        return Err(RpcError::internal_error()
            .with_message(format!("resource URI not found: {}", req.params.uri)));
    };

    Ok(ReadResourceResult {
        contents: vec![content],
        meta: None,
    })
}

pub fn list_prompts_request(_req: ListPromptsRequest) -> Result<ListPromptsResult, RpcError> {
    Ok(ListPromptsResult {
        meta: None,
        next_cursor: None,
        prompts: crate::prompts::get_all(),
    })
}

pub fn get_prompt_request(req: GetPromptRequest) -> Result<GetPromptResult, RpcError> {
    let Some(result) = crate::prompts::call(&req.params.name, req.params.arguments.as_ref()) else {
        return Err(RpcError::internal_error()
            .with_message(format!("prompt not found: {}", req.params.name)));
    };

    Ok(result)
}

pub fn list_tools_request(_req: ListToolsRequest) -> Result<ListToolsResult, RpcError> {
    Ok(ListToolsResult {
        meta: None,
        next_cursor: None,
        tools: crate::tools::get_all(),
    })
}

pub fn call_tool_request(req: CallToolRequest) -> Result<CallToolResult, RpcError> {
    let Some(result) = crate::tools::call(&req.params.name, req.params.arguments.as_ref()) else {
        return Err(
            RpcError::internal_error().with_message(format!("tool not found: {}", req.params.name))
        );
    };

    Ok(result)
}
