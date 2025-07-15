import sys, os
sys.path.append(os.path.join(os.path.dirname(__file__), "lib"))
from repo_finder import find_program_repos

def main():
    if len(sys.argv) < 2:
        print("Usage: python anchor_repo_search.py <output_file>")
        sys.exit(1)

    # TODO: filter repo names containing: 
    # ["ctf", "anchor", "example", "bootcamp", "solana", "sealevel"]
    find_program_repos(
        dep_query='anchor-lang filename:Cargo.toml',
        code_snippet='#[program]',
        label='Anchor',
        output_file=sys.argv[1]
    )

if __name__ == '__main__':
    main()
