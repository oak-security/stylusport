use std::collections::HashMap;

use rust_mcp_schema::{ContentBlock, GetPromptResult, Prompt, PromptMessage, Role, TextContent};

pub fn get_all() -> Vec<Prompt> {
    vec![Prompt {
        arguments: vec![],
        description: Some("Prompts an LLM agent to plan for Solana program migration with the aid of the StylusPort::Solana handbook and MPC server".to_owned()),
        meta: None,
        name: "plan_solana_program_stylus_migration".to_owned(),
        title: Some("Plan Solana Program Migration to Stylus".to_owned()),
    }]
}

fn plan_solana_program_stylus_migration(
    _args: Option<&HashMap<String, String>>,
) -> GetPromptResult {
    GetPromptResult {
        description: None,
        messages: vec![PromptMessage {
            content: ContentBlock::TextContent(TextContent::new(
                include_str!("prompts/plan_solana_program_stylus_migration.md").to_owned(),
                None,
                None,
            )),
            role: Role::User,
        }],
        meta: None,
    }
}

pub fn call(name: &str, args: Option<&HashMap<String, String>>) -> Option<GetPromptResult> {
    match name {
        "plan_solana_program_stylus_migration" => Some(plan_solana_program_stylus_migration(args)),
        _ => None,
    }
}
