use rust_mcp_schema::{CallToolResult, ContentBlock, TextContent, Tool, ToolInputSchema};

#[derive(Debug, PartialEq, Eq)]
pub struct CallResponse {
    content: Vec<String>,
    is_error: bool,
}

impl CallResponse {
    fn success(content: impl Into<String>) -> Self {
        Self {
            content: vec![content.into()],
            is_error: false,
        }
    }

    fn error(msg: impl Into<String>) -> Self {
        Self {
            content: vec![msg.into()],
            is_error: true,
        }
    }
}

impl From<CallResponse> for CallToolResult {
    fn from(value: CallResponse) -> Self {
        Self {
            content: value
                .content
                .into_iter()
                .map(|text| TextContent::new(text, None, None))
                .map(ContentBlock::TextContent)
                .collect(),
            is_error: Some(value.is_error),
            meta: None,
            structured_content: None,
        }
    }
}

macro_rules! define_tool {
    (
        $fn_name:ident,
        description: $description:literal,
        title: $title:literal,
        params: {
            $(
                $param_name:ident: $param_type:literal = $param_desc:literal
            ),+ $(,)?
        },
        handler: $handler:path
    ) => {
        pub mod $fn_name {
            use super::*;

            pub fn tool() -> Tool {
                let mut properties = ::std::collections::HashMap::new();
                $(
                    let mut param_map = ::serde_json::Map::new();
                    param_map.insert("type".to_owned(), ::serde_json::Value::String($param_type.to_owned()));
                    param_map.insert("description".to_owned(), ::serde_json::Value::String($param_desc.to_owned()));
                    properties.insert(stringify!($param_name).to_owned(), param_map);
                )+

                Tool {
                    annotations: None,
                    description: Some($description.to_owned()),
                    input_schema: ToolInputSchema::new(
                        vec![$(stringify!($param_name).to_owned()),+],
                        Some(properties),
                    ),
                    meta: None,
                    name: stringify!($fn_name).to_owned(),
                    output_schema: None,
                    title: Some($title.to_owned()),
                }
            }

            pub fn call(arguments: Option<&::serde_json::Map<String, ::serde_json::Value>>) -> CallResponse {
                $(
                    let Some($param_name) = arguments
                        .and_then(|map| map.get(stringify!($param_name)))
                        .and_then(|value| value.as_str())
                    else {
                        return CallResponse::error(concat!(stringify!($param_name), " argument missing"));
                    };
                )+

                $handler($($param_name),+)
            }
        }
    };
}

macro_rules! define_tools {
    (
        $(
            $fn_name:ident => {
                description: $description:literal,
                title: $title:literal,
                params: {
                    $(
                        $param_name:ident: $param_type:literal = $param_desc:literal
                    ),+ $(,)?
                },
            }
        ),+ $(,)?
    ) => {
        $(
            define_tool!(
                $fn_name,
                description: $description,
                title: $title,
                params: {
                    $($param_name: $param_type = $param_desc),+
                },
                handler: $fn_name
            );
        )+

        pub fn get_all() -> Vec<Tool> {
            vec![
                $(
                    $fn_name::tool(),
                )+
            ]
        }

        pub fn call(name: &str, arguments: Option<&::serde_json::Map<String, ::serde_json::Value>>) -> Option<CallToolResult> {
            match name {
                $(
                    stringify!($fn_name) => Some($fn_name::call(arguments).into()),
                )+
                _ => None,
            }
        }
    };
}

fn detect_solana_program_kind(cargo_manifest: &str) -> CallResponse {
    if !cargo_manifest.contains("[package]") || !cargo_manifest.contains("dependencies") {
        return CallResponse::error("invalid cargo manifest file");
    }

    if cargo_manifest.contains("anchor-lang") {
        return CallResponse::success("anchor");
    }

    if cargo_manifest.contains("solana-program") {
        return CallResponse::success("native");
    }

    CallResponse::error(
        "no solana program kind detected, are you sure this is the correct Cargo.toml file?",
    )
}

// https://doc.rust-lang.org/cargo/reference/manifest.html#the-name-field
fn invalid_package_name(package_name: &str) -> bool {
    package_name.is_empty()
        || package_name.len() > 64
        || !package_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

static INVALID_PACKAGE_NAME_MSG: &str = "invalid package name - must be non-empty, not longer than 64 characters and only contain ASCII alphanumerics, hyphens & underscores";

fn generate_stylus_contract_cargo_manifest(package_name: &str) -> CallResponse {
    if invalid_package_name(package_name) {
        return CallResponse::error(INVALID_PACKAGE_NAME_MSG);
    }

    let alloy_primitives_version = std::env::var("STYLUSPORT_MCP_ALLOY_PRIMITIVES_VERSION")
        .unwrap_or_else(|_| "=0.8.20".to_owned());
    let alloy_sol_types_version = std::env::var("STYLUSPORT_MCP_ALLOY_SOL_TYPES_VERSION")
        .unwrap_or_else(|_| "=0.8.20".to_owned());
    let openzeppelin_stylus_version = std::env::var("STYLUSPORT_MCP_OPENZEPPELIN_STYLUS_VERSION")
        .unwrap_or_else(|_| "0.3.0".to_owned());
    let stylus_sdk_version =
        std::env::var("STYLUSPORT_MCP_STYLUS_SDK_VERSION").unwrap_or_else(|_| "=0.9.0".to_owned());
    let arbitrary_version =
        std::env::var("STYLUSPORT_MCP_ARBITRARY_VERSION").unwrap_or_else(|_| "=1.4.2".to_owned());
    let motsu_version =
        std::env::var("STYLUSPORT_MCP_MOTSU_VERSION").unwrap_or_else(|_| "0.10.0".to_owned());

    CallResponse::success(format!(
        r#"
[package]
name = "{package_name}"
version = "0.1.0"
edition = "2021"

[features]
export-abi = ["stylus-sdk/export-abi", "openzeppelin-stylus/export-abi"]

[dependencies]
alloy-primitives = "{alloy_primitives_version}"
alloy-sol-types = "{alloy_sol_types_version}"
openzeppelin-stylus = "{openzeppelin_stylus_version}"
stylus-sdk = "{stylus_sdk_version}"

[dev-dependencies]
alloy-primitives = {{ version = "{alloy_primitives_version}", features = [ "tiny-keccak" ] }}
arbitrary = {{ version = "{arbitrary_version}", features = [ "derive" ] }}
motsu = "{motsu_version}""#
    ))
}

fn generate_stylus_contract_main_rs(package_name: &str) -> CallResponse {
    if invalid_package_name(package_name) {
        return CallResponse::error(INVALID_PACKAGE_NAME_MSG);
    }

    let package_name = package_name.replace("-", "_");

    CallResponse::success(format!(
        r#"
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]

#[cfg(not(any(test, feature = "export-abi")))]
#[no_mangle]
pub extern "C" fn main() {{}}

#[cfg(feature = "export-abi")]
fn main() {{
    {package_name}::print_from_args();
}}
"#
    ))
}

fn search_handbook(query: &str) -> CallResponse {
    if query.is_empty() {
        return CallResponse::error("query cannot be an empty string");
    }

    CallResponse {
        content: crate::resources::search(query),
        is_error: false,
    }
}

define_tools! {
    detect_solana_program_kind => {
        description: "Detect the kind of a Solana program, either 'native' or 'anchor', from its Cargo.toml file",
        title: "Detect Solana Program Kind",
        params: {
            cargo_manifest: "string" = "Solana program Cargo.toml file",
        },
    },
    generate_stylus_contract_cargo_manifest => {
        description: "Generate the Cargo.toml file for a Stylus contract",
        title: "Generate Stylus Contract Cargo.toml",
        params: {
            package_name: "string" = "Stylus contract package name",
        },
    },
    generate_stylus_contract_main_rs => {
        description: "Generate the main.rs file for a Stylus contract",
        title: "Generate Stylus Contract main.rs",
        params: {
            package_name: "string" = "Stylus contract package name",
        },
    },
    search_handbook => {
        description: "Search the StylusPort::Solana Handbook, receiving a list of resource URIs in descending order of relevance score",
        title: "Search StylusPort::Solana Handbook",
        params: {
            query: "string" = "Search query",
        },
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_package_name_valid_names() {
        assert!(!invalid_package_name("my_package"));
        assert!(!invalid_package_name("my-package"));
        assert!(!invalid_package_name("package123"));
        assert!(!invalid_package_name("a"));
        assert!(!invalid_package_name("Package_Name-123"));
        assert!(!invalid_package_name("_underscore"));
        assert!(!invalid_package_name("-dash"));
    }

    #[test]
    fn test_invalid_package_name_empty_name() {
        assert!(invalid_package_name(""));
    }

    #[test]
    fn test_invalid_package_name_length_boundary() {
        let valid_64 = "a".repeat(64);
        assert!(!invalid_package_name(&valid_64));
        let invalid_65 = "a".repeat(65);
        assert!(invalid_package_name(&invalid_65));
        let invalid_long = "a".repeat(100);
        assert!(invalid_package_name(&invalid_long));
    }

    #[test]
    fn test_invalid_package_name_invalid_characters() {
        assert!(invalid_package_name("my package")); // space
        assert!(invalid_package_name("my.package")); // dot
        assert!(invalid_package_name("my@package")); // @
        assert!(invalid_package_name("my/package")); // slash
        assert!(invalid_package_name("my\\package")); // backslash
        assert!(invalid_package_name("my!package")); // exclamation
        assert!(invalid_package_name("my#package")); // hash
        assert!(invalid_package_name("my$package")); // dollar
        assert!(invalid_package_name("my%package")); // percent
        assert!(invalid_package_name("my_package\n")); // newline
                                                       // assert!(invalid_package_name("mypackage"\n[dependencies]\nmalicious-crate = \"1.0\"")) // newlines
    }

    #[test]
    fn test_invalid_package_name_unicode() {
        assert!(invalid_package_name("my_packagé")); // accented char
        assert!(invalid_package_name("my_package™")); // symbol
        assert!(invalid_package_name("my_📦")); // emoji
    }

    #[test]
    fn test_generate_stylus_contract_main_rs_rejects_code_injection() {
        assert_eq!(
            generate_stylus_contract_main_rs("mypackage\n malicious_code();\n"),
            CallResponse::error(INVALID_PACKAGE_NAME_MSG)
        )
    }

    #[test]
    fn test_generate_stylus_contract_cargo_manifest_rejects_malicious_dependency_injection() {
        assert_eq!(
            generate_stylus_contract_cargo_manifest(
                "mypackage\n[dependencies]\nmalicious-crate = \"1.0\""
            ),
            CallResponse::error(INVALID_PACKAGE_NAME_MSG)
        )
    }
}
