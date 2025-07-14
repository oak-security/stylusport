#!/usr/bin/env python3
import sys
from repo_finder import find_program_repos

def main():
    if len(sys.argv) < 2:
        print("Usage: python solana_repo_search.py <output_file>")
        sys.exit(1)

    # TODO: filter repo names containing: 
    # ["ctf", "solana", "fork", "agave", "wormhole", "rust", "hello", "test", "anchor", "learn", "challenge", "chain"]
    find_program_repos(
        dep_query='solana-program filename:Cargo.toml',
        code_snippet='entrypoint!',
        label='Solana',
        output_file=sys.argv[1]
    )

if __name__ == '__main__':
    main()
