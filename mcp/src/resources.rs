mod bm25;

macro_rules! create_chapter {
    ($module_name:ident, $file_path:literal, $title:literal, $description:literal) => {
        pub mod $module_name {
            pub static CONTENT: &str = include_str!(concat!("../../", $file_path));

            pub fn resource() -> ::rust_mcp_schema::Resource {
                ::rust_mcp_schema::Resource {
                    annotations: None,
                    description: Some($description.to_owned()),
                    meta: None,
                    mime_type: Some("text/markdown".to_owned()),
                    name: stringify!($module_name).replace('_', "-").to_owned(),
                    size: Some(CONTENT.len() as _),
                    title: Some($title.to_owned()),
                    uri: concat!("file://", $file_path).to_owned(),
                }
            }

            pub fn content() -> ::rust_mcp_schema::ReadResourceResultContentsItem {
                ::rust_mcp_schema::TextResourceContents {
                    meta: None,
                    mime_type: Some("text/markdown".to_owned()),
                    text: CONTENT.to_owned(),
                    uri: concat!("file://", $file_path).to_owned(),
                }
                .into()
            }
        }
    };
}

macro_rules! create_handbook {
    (
        $(
            $module_name:ident => {
                file_path: $file_path:literal,
                title: $title:literal,
                description: $description:literal
            }
        ),+ $(,)?
    ) => {
        pub mod chapter {
            $(
                create_chapter!($module_name, $file_path, $title, $description);
            )+
        }

        static INDEX: std::sync::LazyLock<bm25::Index<'static>> = std::sync::LazyLock::new(|| {
            let mut index = bm25::Index::new(1.2);
            $(
                index.add_doc(
                    concat!("file://", $file_path),
                    chapter::$module_name::CONTENT
                );
            )+
            index.finalize();
            index
        });

        pub fn search(query: &str) -> Vec<String> {
            INDEX.score(query)
                .into_iter()
                .map(|(uri, _score)| uri.to_string())
                .collect()
        }

        pub fn get_resource(uri: &str) -> Option<::rust_mcp_schema::ReadResourceResultContentsItem> {
            match uri {
                $(
                    concat!("file://", $file_path) => Some(chapter::$module_name::content()),
                )+
                _ => None,
            }
        }

        pub fn get_all() -> Vec<::rust_mcp_schema::Resource> {
            vec![
                $(
                    chapter::$module_name::resource(),
                )+
            ]
        }
    };
}

create_handbook! {
    introduction => {
        file_path: "/handbook/src/introduction.md",
        title: "Introduction",
        description: "Introduction to migrating from Solana to Stylus"
    },
    program_structure => {
        file_path: "/handbook/src/program-structure.md",
        title: "Handbook Chapter: Program Structure",
        description: "Comparing the program structure of Solana programs to Stylus Contracts"
    },
    state_storage => {
        file_path: "/handbook/src/state-storage.md",
        title: "Handbook Chapter: State Storage Patterns",
        description: "Patterns for storing state in Stylus vs Solana"
    },
    access_control => {
        file_path: "/handbook/src/access-control.md",
        title: "Handbook Chapter: Access Control Migration",
        description: "Migrating access control patterns from Solana to Stylus"
    },
    external_calls => {
        file_path: "/handbook/src/external-calls.md",
        title: "Handbook Chapter: External Program Calls",
        description: "Making calls to external programs in Stylus compared to Solana"
    },
    native_tokens => {
        file_path: "/handbook/src/native-tokens.md",
        title: "Handbook Chapter: Native Token Operations",
        description: "Working with native tokens in Stylus compared to Solana"
    },
    fungible_tokens => {
        file_path: "/handbook/src/fungible-tokens.md",
        title: "Handbook Chapter: Fungible Token Handling",
        description: "Handling fungible tokens in Stylus contracts compared to Solana"
    },
    non_fungible_tokens => {
        file_path: "/handbook/src/non-fungible-tokens.md",
        title: "Handbook Chapter: Non-Fungible Token Handling",
        description: "Working with NFTs in Stylus compared to Solana"
    },
    errors_events => {
        file_path: "/handbook/src/errors-events.md",
        title: "Handbook Chapter: Errors and Events",
        description: "Error handling and event emission in Stylus compared to Solana"
    },
    case_study_bonafida_token_vesting => {
        file_path: "/handbook/src/case-study-bonafida-token-vesting.md",
        title: "Case Study: Migrating Bonafida's Token Vesting to Stylus",
        description: "A complete case study of migrating Bonafida's Token Vesting contract from Solana to Stylus"
    },
    testing_debugging => {
        file_path: "/handbook/src/testing-debugging.md",
        title: "Handbook Chapter: Testing and Debugging",
        description: "Testing and debugging techniques for Stylus contracts compared to Solana programs"
    },
    gas_optimization => {
        file_path: "/handbook/src/gas-optimization.md",
        title: "Handbook Chapter: Gas Optimization",
        description: "Gas optimization considerations for Stylus contracts compared to Solana programs"
    },
    security_considerations => {
        file_path: "/handbook/src/security-considerations.md",
        title: "Handbook Chapter: Security Considerations",
        description: "Security considerations when developing Stylus contracts compared to Solana programs"
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_returns_results() {
        let results = search("constructor");
        assert!(
            !results.is_empty(),
            "should find chapters mentioning constructors"
        );
    }

    #[test]
    fn search_returns_valid_uris() {
        let results = search("Solana Stylus");
        assert!(!results.is_empty(), "should find chapters about migration");

        for uri in results {
            assert!(
                uri.starts_with("file://"),
                "URI should have file:// prefix: {uri}",
            );
            assert!(
                get_resource(&uri).is_some(),
                "URI should be retrievable: {uri}",
            );
        }
    }

    #[test]
    fn search_with_code_identifiers() {
        let results = search("StorageAddress msg_sender");
        assert!(
            !results.is_empty(),
            "should find chapters with these identifiers"
        );
    }

    #[test]
    fn search_empty_query_returns_empty() {
        let results = search("");
        assert!(results.is_empty());
    }

    #[test]
    fn search_returns_multiple_results() {
        let results = search("token");
        assert!(
            results.len() >= 2,
            "should find multiple chapters discussing tokens"
        );
    }
}
