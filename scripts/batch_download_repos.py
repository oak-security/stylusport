#!/usr/bin/env python3
import json
import subprocess
import sys
import time
import tarfile
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
from dotenv import load_dotenv
import os

# Load environment variables
load_dotenv()

GITHUB_TOKEN = os.getenv('GITHUB_ACCESS_TOKEN')
if not GITHUB_TOKEN:
    raise ValueError("GITHUB_ACCESS_TOKEN not found in .env file")


def load_repos(json_file):
    """Load repository data from JSON file."""
    with open(json_file, 'r') as f:
        return json.load(f)


def download_tarball(url, owner, repo, output_file, max_retries=3, delay=2):
    """Download repository tarball using GitHub API."""
    
    def build_curl_command(branch):
        return [
            'curl', '-L', '--output', str(output_file),
            '-H', 'Accept: application/vnd.github+json',
            '-H', f'Authorization: Bearer {GITHUB_TOKEN}',
            '-H', 'X-GitHub-Api-Version: 2022-11-28',
            f"https://api.github.com/repos/{owner}/{repo}/tarball/{branch}"
        ]

    def is_valid_download():
        if not output_file.exists() or output_file.stat().st_size == 0:
            return False
        try:
            with open(output_file, 'r', encoding='utf-8', errors='ignore') as f:
                if '404: Not Found' in f.read(100):
                    output_file.unlink()
                    return False
        except:
            pass  # Binary file — assume OK
        return True

    def attempt_download(branch):
        for attempt in range(max_retries):
            try:
                result = subprocess.run(build_curl_command(branch), capture_output=True, text=True)
                if result.returncode == 0 and is_valid_download():
                    print(f"✓ Downloaded: {owner}/{repo} (branch: {branch})")
                    return True

                if 'rate limit' in result.stderr.lower() or result.returncode == 22:
                    if attempt < max_retries - 1:
                        wait_time = delay * (2 ** attempt)
                        print(f"⚠ Rate limited, retrying in {wait_time}s... (attempt {attempt + 1}/{max_retries})")
                        time.sleep(wait_time)
                        continue

            except Exception as e:
                print(f"✗ Error downloading {owner}/{repo} (branch: {branch}): {e}")
                break
        return False

    for branch in ['main', 'master']:
        if attempt_download(branch):
            return True
        print(f"⚠ Branch '{branch}' not successful for {owner}/{repo}, trying next...")

    print(f"✗ Failed to download {owner}/{repo}: neither 'main' nor 'master' branch found")
    return False

def extract_tarball(tar_path, extract_dir):
    """Extract tarball and remove it."""
    try:
        with tarfile.open(tar_path, 'r:gz') as tar:
            # Extract to temporary directory first
            temp_dir = extract_dir.parent / f"{extract_dir.name}_temp"
            tar.extractall(temp_dir, filter='data')
            
            # Find the extracted directory (GitHub tarballs have a single root directory)
            extracted_dirs = [d for d in temp_dir.iterdir() if d.is_dir()]
            if extracted_dirs:
                # Rename the extracted directory to our desired name
                extracted_dirs[0].rename(extract_dir)
                temp_dir.rmdir()
            else:
                temp_dir.rename(extract_dir)
        
        # Remove the tarball
        tar_path.unlink()
        print(f"✓ Extracted: {extract_dir.name}")
        return True
        
    except Exception as e:
        print(f"✗ Error extracting {tar_path}: {e}")
        return False

def main():
    if len(sys.argv) != 4:
        print("Usage: python batch_download_repos.py <json_file> <num_entries> <output_path>")
        sys.exit(1)
    
    json_file, num_entries, output_path = sys.argv[1], int(sys.argv[2]), Path(sys.argv[3])
    
    repos = load_repos(json_file)
    output_path.mkdir(parents=True, exist_ok=True)
    
    print(f"Downloading {num_entries} repositories to '{output_path}'...")
    
    successful_downloads = failed_downloads = 0
    download_tasks = []
    
    # Download phase
    for idx, repo in enumerate(repos[:num_entries]):
        try:
            owner, repo_name = repo['name'].split('/')
            tar_filename = f"{idx}-{owner}.{repo_name}.tar"
            tar_path = output_path / tar_filename
            extract_dir = output_path / f"{idx}-{owner}-{repo_name}"
            
            print(f"\n[{idx + 1}/{num_entries}] Processing: {repo['name']}")
            
            # Skip if already extracted
            if extract_dir.exists():
                print(f"⚠ Directory already exists, skipping...")
                successful_downloads += 1
                continue
            
            # Skip if tarball already exists
            if tar_path.exists():
                print(f"⚠ Tarball already exists, will extract...")
                download_tasks.append((tar_path, extract_dir))
                successful_downloads += 1
                continue
            
            if download_tarball(repo['url'], owner, repo_name, tar_path):
                download_tasks.append((tar_path, extract_dir))
                successful_downloads += 1
            else:
                failed_downloads += 1
                
        except Exception as e:
            print(f"✗ Error processing {repo['name']}: {e}")
            failed_downloads += 1
    
    # Extraction phase - parallel
    if download_tasks:
        print(f"\n{'='*50}")
        print(f"Extracting {len(download_tasks)} tarballs in parallel...")
        
        with ThreadPoolExecutor(max_workers=4) as executor:
            futures = [executor.submit(extract_tarball, tar_path, extract_dir) 
                      for tar_path, extract_dir in download_tasks]
            
            extraction_results = [future.result() for future in futures]
    
    print(f"\n{'='*50}")
    print(f"✓ Successful: {successful_downloads}")
    print(f"✗ Failed: {failed_downloads}")
    
    if failed_downloads > 0:
        sys.exit(1)

if __name__ == "__main__":
    main()
