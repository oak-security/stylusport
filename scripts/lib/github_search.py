import os
import time
import requests
from dotenv import load_dotenv

# Load environment variables
load_dotenv()

GITHUB_TOKEN = os.getenv('GITHUB_ACCESS_TOKEN')
if not GITHUB_TOKEN:
    raise ValueError("GITHUB_ACCESS_TOKEN not found in .env file")

HEADERS = {
    'Authorization': f'token {GITHUB_TOKEN}',
    'Accept': 'application/vnd.github.v3+json'
}

def wait_for_rate_limit(response):
    retry_after = response.headers.get('Retry-After')
    if retry_after:
        wait_time = int(retry_after) + 1
        print(f"Rate limit hit. Waiting {wait_time} seconds (Retry-After header)...")
        time.sleep(wait_time)
        return

    remaining = int(response.headers.get('X-RateLimit-Remaining', 0))
    if remaining == 0:
        reset_time = int(response.headers.get('X-RateLimit-Reset', time.time()))
        wait_time = reset_time - int(time.time()) + 1
        if wait_time > 0:
            print(f"Primary rate limit exhausted. Waiting {wait_time} seconds...")
            time.sleep(wait_time)
    else:
        print("Secondary rate limit hit. Waiting 60 seconds...")
        time.sleep(60)

def search_github(endpoint, query, max_results=1000):
    results = []
    page = 1
    max_results = min(max_results, 1000)

    while len(results) < max_results:
        url = f"https://api.github.com/search/{endpoint}"
        params = {'q': query, 'per_page': 100, 'page': page}

        response = requests.get(url, headers=HEADERS, params=params)
        if response.status_code in [403, 429]:
            wait_for_rate_limit(response)
            continue

        response.raise_for_status()
        data = response.json()

        if not data.get('items'):
            break

        results.extend(data['items'])

        if len(results) >= 1000 or len(data['items']) < 100:
            break

        page += 1
        time.sleep(3)

    return results[:max_results]

def get_repo_details(full_name):
    url = f"https://api.github.com/repos/{full_name}"
    while True:
        response = requests.get(url, headers=HEADERS)
        if response.status_code in [403, 429]:
            wait_for_rate_limit(response)
            continue
        response.raise_for_status()
        return response.json()
