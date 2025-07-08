import json
import time
from github_search import search_github, get_repo_details

def find_program_repos(dep_query, code_snippet, label, output_file):
    print(f"Searching for {label} repositories...")

    cargo_results = search_github('code', dep_query, max_results=1000)
    print(f"Found {len(cargo_results)} Cargo.toml files with {dep_query}")

    unique_repos = {}
    for item in cargo_results:
        repo = item['repository']
        if not repo['fork']:
            full_name = repo['full_name']
            unique_repos[full_name] = repo

    print(f"Found {len(unique_repos)} unique non-forked repositories")

    matched_repos = []
    for i, (full_name, _) in enumerate(unique_repos.items()):
        print(f"Checking {full_name} ({i+1}/{len(unique_repos)})...")
        code_query = f'repo:{full_name} "{code_snippet}" extension:rs'

        try:
            code_results = search_github('code', code_query, max_results=1)
            if code_results:
                repo_details = get_repo_details(full_name)
                matched_repos.append({
                    'url': repo_details['html_url'],
                    'stars': repo_details['stargazers_count'],
                    'last_commit': repo_details['pushed_at'],
                    'name': repo_details['full_name'],
                    'description': repo_details.get('description', '')
                })
                print(f"  ✓ Found {label} program: {full_name}")
        except Exception as e:
            print(f"  Error checking {full_name}: {e}")
        time.sleep(3)

    matched_repos.sort(key=lambda x: (x['stars'], x['last_commit']), reverse=True)
    with open(output_file, 'w') as f:
        json.dump(matched_repos, f, indent=2)

    print(f"\nFound {len(matched_repos)} {label} repositories")
    print(f"Results saved to {output_file}")
    if matched_repos:
        print("\nTop repositories:")
        for repo in matched_repos[:10]:
            print(f"- {repo['name']} ({repo['stars']} stars)")
