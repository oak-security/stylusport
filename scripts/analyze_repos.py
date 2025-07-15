#!/usr/bin/env python3

import os
import sys
import json
import argparse
import asyncio
import time
from pathlib import Path
from typing import Dict, List, Optional, Tuple, NamedTuple
from collections import deque
import toml
from openai import OpenAI
from dotenv import load_dotenv
from tqdm import tqdm
import tiktoken


class ProgramInfo(NamedTuple):
    """Information about a program to be analyzed."""
    cargo_file: Path
    dependencies: Dict
    package_name: str
    repo_name: str


class TokenRateLimiter:
    """Token-based rate limiter that tracks tokens per minute."""
    
    def __init__(self, tokens_per_minute: int, model: str = "gpt-3.5-turbo"):
        self.tokens_per_minute = tokens_per_minute
        self.token_timestamps = deque()  # (timestamp, token_count) pairs
        self.lock = asyncio.Lock()
        
        # Initialize tokenizer for the model
        try:
            self.encoding = tiktoken.encoding_for_model(model)
        except KeyError:
            # Fallback to cl100k_base encoding for unknown models
            self.encoding = tiktoken.get_encoding("cl100k_base")
    
    def estimate_tokens(self, messages: List[Dict[str, str]]) -> int:
        """Estimate token count for a list of messages."""
        total_tokens = 0
        
        for message in messages:
            # Add tokens for role and content
            total_tokens += len(self.encoding.encode(message.get("role", "")))
            total_tokens += len(self.encoding.encode(message.get("content", "")))
            # Add some overhead for message formatting
            total_tokens += 4
        
        # Add overhead for the API call structure
        total_tokens += 10
        
        return total_tokens
    
    def _cleanup_old_tokens(self, current_time: float):
        """Remove token records older than 1 minute."""
        while self.token_timestamps and current_time - self.token_timestamps[0][0] > 60:
            self.token_timestamps.popleft()
    
    def _current_tokens_in_window(self, current_time: float) -> int:
        """Calculate current token usage in the last minute."""
        self._cleanup_old_tokens(current_time)
        return sum(tokens for _, tokens in self.token_timestamps)
    
    async def wait_for_capacity(self, estimated_tokens: int) -> float:
        """Wait until there's capacity for the estimated tokens, return wait time."""
        async with self.lock:
            current_time = time.time()
            current_usage = self._current_tokens_in_window(current_time)
            
            if current_usage + estimated_tokens <= self.tokens_per_minute:
                # We have capacity, record the usage
                self.token_timestamps.append((current_time, estimated_tokens))
                return 0.0
            
            # We need to wait - find when the oldest tokens will expire
            if not self.token_timestamps:
                # No previous usage, we can proceed
                self.token_timestamps.append((current_time, estimated_tokens))
                return 0.0
            
            # Calculate how long to wait for enough capacity
            needed_capacity = estimated_tokens
            temp_usage = current_usage
            wait_until = current_time
            
            # Find when we'll have enough capacity by removing old tokens
            temp_timestamps = list(self.token_timestamps)
            while temp_usage + needed_capacity > self.tokens_per_minute and temp_timestamps:
                oldest_time, oldest_tokens = temp_timestamps.pop(0)
                wait_until = max(wait_until, oldest_time + 60)
                temp_usage -= oldest_tokens
            
            wait_time = max(0, wait_until - current_time)
            
            if wait_time > 0:
                await asyncio.sleep(wait_time)
                # Record the usage after waiting
                self.token_timestamps.append((time.time(), estimated_tokens))
            else:
                self.token_timestamps.append((current_time, estimated_tokens))
            
            return wait_time


def find_cargo_toml_files(directory: Path) -> List[Path]:
    """Find all Cargo.toml files in the given directory and its subdirectories."""
    cargo_files = []
    for root, dirs, files in os.walk(directory):
        if 'Cargo.toml' in files:
            cargo_files.append(Path(root) / 'Cargo.toml')
    return cargo_files


def read_dependencies(cargo_toml_path: Path) -> Optional[Dict]:
    """Read the [dependencies] section from a Cargo.toml file."""
    try:
        with open(cargo_toml_path, 'r', encoding='utf-8') as f:
            cargo_data = toml.load(f)
        return cargo_data.get('dependencies', {})
    except Exception as e:
        print(f"Error reading {cargo_toml_path}: {e}", file=sys.stderr)
        return None


def check_dependency_exists(dependencies: Dict, target_dependency: str) -> bool:
    """Check if the target dependency exists in the dependencies dict."""
    return target_dependency in dependencies


def get_package_name(cargo_file: Path, repo_root_dir: Path) -> str:
    """Get package name from cargo file path relative to repo root."""
    package_rel_path = cargo_file.relative_to(repo_root_dir)
    if package_rel_path.name == "Cargo.toml":
        package_name = str(package_rel_path.parent).replace("/", "_").replace("\\", "_")
        if package_name == ".":
            package_name = "root"
    else:
        package_name = str(package_rel_path.parent).replace("/", "_").replace("\\", "_")
    return package_name


def find_programs_to_analyze(root_path: Path, target_dependency: str, 
                           exclude_dependency: Optional[str] = None) -> List[ProgramInfo]:
    """Find all programs that match the filtering criteria."""
    programs_to_analyze = []
    
    print("Scanning for programs to analyze...")
    
    # Process each subdirectory (repository)
    for repo_dir in tqdm(list(root_path.iterdir()), desc="Scanning repositories"):
        if not repo_dir.is_dir():
            continue
            
        repo_name = repo_dir.name
        cargo_files = find_cargo_toml_files(repo_dir)
        
        for cargo_file in cargo_files:
            dependencies = read_dependencies(cargo_file)
            
            if dependencies is None:
                continue
            
            # Skip if exclude_dependency is present
            if exclude_dependency and check_dependency_exists(dependencies, exclude_dependency):
                continue
                
            if check_dependency_exists(dependencies, target_dependency):
                package_name = get_package_name(cargo_file, repo_dir)
                
                programs_to_analyze.append(ProgramInfo(
                    cargo_file=cargo_file,
                    dependencies=dependencies,
                    package_name=package_name,
                    repo_name=repo_name
                ))
    
    return programs_to_analyze


def find_rust_files(cargo_dir: Path) -> List[Path]:
    """Find all .rs files in the same directory and subdirectories as the Cargo.toml file."""
    rust_files = []
    for root, dirs, files in os.walk(cargo_dir):
        for file in files:
            if file.endswith('.rs'):
                rust_files.append(Path(root) / file)
    return sorted(rust_files)


def read_system_prompt(system_prompt_path: Path) -> str:
    """Read the system prompt from file."""
    try:
        with open(system_prompt_path, 'r', encoding='utf-8') as f:
            return f.read().strip()
    except Exception as e:
        print(f"Error reading system prompt file {system_prompt_path}: {e}", file=sys.stderr)
        sys.exit(1)


def read_rust_file(file_path: Path) -> Optional[str]:
    """Read contents of a Rust file."""
    try:
        with open(file_path, 'r', encoding='utf-8') as f:
            return f.read()
    except Exception as e:
        print(f"Error reading {file_path}: {e}", file=sys.stderr)
        return None


async def summarize_file_with_llm(client: OpenAI, system_prompt: str, model: str, file_path: Path, 
                                file_content: str, rate_limiter: TokenRateLimiter) -> Optional[str]:
    """Summarize a single file using the LLM with rate limiting."""
    try:
        user_prompt = f"""File path: {file_path}

File contents:
```rust
{file_content}
```

Please provide a concise summary of this Rust file's purpose and functionality."""

        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ]
        
        # Estimate tokens and wait for capacity
        estimated_tokens = rate_limiter.estimate_tokens(messages)
        # Add estimated response tokens (500 max_tokens)
        estimated_tokens += 500
        
        wait_time = await rate_limiter.wait_for_capacity(estimated_tokens)
        # if wait_time > 0:
        #     print(f"  Waited {wait_time:.1f}s for rate limit (estimated {estimated_tokens} tokens)")

        response = client.chat.completions.create(
            model=model,
            messages=messages,
            max_tokens=500,
            temperature=0.1
        )
        
        return response.choices[0].message.content.strip()
    
    except Exception as e:
        print(f"Error summarizing {file_path}: {e}", file=sys.stderr)
        return None


async def generate_program_report(client: OpenAI, system_prompt: str, model: str, cargo_toml_path: Path, 
                                package_name: str, dependencies: Dict, file_summaries: List[Tuple[str, str]],
                                rate_limiter: TokenRateLimiter) -> Optional[str]:
    """Generate a comprehensive report for a single program with rate limiting."""
    try:
        dependencies_str = json.dumps(dependencies, indent=2)
        
        files_section = "\n\n".join([
            f"**File: {rel_path}**\n{summary}" 
            for rel_path, summary in file_summaries
        ])
        
        user_prompt = f"""Program: {cargo_toml_path}

Dependencies:
```json
{dependencies_str}
```

File Summaries:
{files_section}

Please provide a concise report about this Solana program package called {package_name}, including:
1. File Tree Diagram with concise in-line one-liner comment stating the files purpose
1.1 The root path in the tree should be {package_name}
3. Dependency List with a concise one-liner comment stating the dependency's purpose
4. Concise summary of the package and what it does.
5. Any notable features or implementation details"""

        messages = [
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_prompt}
        ]
        
        # Estimate tokens and wait for capacity
        estimated_tokens = rate_limiter.estimate_tokens(messages)
        # Add estimated response tokens (1500 max_tokens)
        estimated_tokens += 1500
        
        wait_time = await rate_limiter.wait_for_capacity(estimated_tokens)
        if wait_time > 0:
            print(f"  Waited {wait_time:.1f}s for rate limit (estimated {estimated_tokens} tokens)")

        response = client.chat.completions.create(
            model=model,
            messages=messages,
            max_tokens=1500,
            temperature=0.1
        )
        
        return response.choices[0].message.content.strip()
    
    except Exception as e:
        print(f"Error generating report for {cargo_toml_path}: {e}", file=sys.stderr)
        return None


async def process_single_program(client: OpenAI, system_prompt: str, model: str, program_info: ProgramInfo, 
                               root_path: Path, pbar: tqdm, rate_limiter: TokenRateLimiter) -> Optional[Dict]:
    """Process a single program and return its report."""
    cargo_file = program_info.cargo_file
    dependencies = program_info.dependencies
    package_name = program_info.package_name
    repo_name = program_info.repo_name
    
    pbar.set_description(f"Processing {repo_name}/{package_name}")
    
    cargo_dir = cargo_file.parent
    rust_files = find_rust_files(cargo_dir)
    
    # Summarize each Rust file
    file_summaries = []
    for rust_file in rust_files:
        file_content = read_rust_file(rust_file)
        if file_content is None:
            continue
        
        # Get relative path from cargo directory
        rel_path = rust_file.relative_to(cargo_dir)
        
        # Create cache filename
        file_path_safe = str(rel_path).replace("/", "_").replace("\\", "_")
        cache_filename = f"{repo_name}-{package_name}-{file_path_safe}.md"
        cache_path = root_path / cache_filename
        
        summary = None
        
        # Check if cached summary exists
        if cache_path.exists():
            try:
                with open(cache_path, 'r', encoding='utf-8') as f:
                    summary = f.read().strip()
            except Exception as e:
                print(f"  Error reading cache file {cache_path}: {e}", file=sys.stderr)
                summary = None
        
        # Generate summary if not cached or cache read failed
        if summary is None:
            summary = await summarize_file_with_llm(client, system_prompt, model, rel_path, file_content, rate_limiter)
            
            # Save summary to cache
            if summary:
                try:
                    with open(cache_path, 'w', encoding='utf-8') as f:
                        f.write(summary)
                except Exception as e:
                    print(f"  Error writing cache file {cache_path}: {e}", file=sys.stderr)
        
        if summary:
            file_summaries.append((str(rel_path), summary))
    
    if not file_summaries:
        pbar.update(1)
        return None
    
    # Check if report already exists
    report_filename = f"{repo_name}-{package_name}-report.md"
    report_path = root_path / report_filename
    
    report = None
    
    if report_path.exists():
        try:
            with open(report_path, 'r', encoding='utf-8') as f:
                report = f.read().strip()
        except Exception as e:
            print(f"  Error reading report file {report_path}: {e}", file=sys.stderr)
            report = None
    
    # Generate report if not cached or cache read failed
    if report is None:
        report = await generate_program_report(client, system_prompt, model, cargo_file, package_name,
                                             dependencies, file_summaries, rate_limiter)
        
        # Save report to cache
        if report:
            try:
                with open(report_path, 'w', encoding='utf-8') as f:
                    f.write(report)
            except Exception as e:
                print(f"  Error writing report file {report_path}: {e}", file=sys.stderr)
    
    pbar.update(1)
    
    if report:
        return {
            "cargo_toml_path": str(cargo_file),
            "dependencies": dependencies,
            "report": report
        }
    
    return None


async def process_programs_with_progress_async(client: OpenAI, system_prompt: str, model: str, 
                                             programs_to_analyze: List[ProgramInfo], root_path: Path,
                                             max_concurrent: int = 4, rate_limiter: TokenRateLimiter = None) -> Dict[str, List[Dict]]:
    """Process all programs with async concurrency control and rate limiting."""
    results_by_repo = {}
    semaphore = asyncio.Semaphore(max_concurrent)
    
    async def process_single_program_async(program_info: ProgramInfo, pbar: tqdm) -> Optional[Dict]:
        """Async wrapper for processing a single program."""
        async with semaphore:
            try:
                result = await process_single_program(
                    client, system_prompt, model, program_info, root_path, pbar, rate_limiter
                )
                return result
            except Exception as e:
                print(f"Error processing {program_info.package_name}: {e}", file=sys.stderr)
                pbar.update(1)  # Still update progress on error
                return None
    
    with tqdm(total=len(programs_to_analyze), desc="Analyzing programs", unit="program") as pbar:
        # Create all tasks
        tasks = [
            process_single_program_async(program_info, pbar)
            for program_info in programs_to_analyze
        ]
        
        # Wait for all tasks to complete
        results = await asyncio.gather(*tasks, return_exceptions=True)
        
        # Process results
        for i, result in enumerate(results):
            if isinstance(result, Exception):
                print(f"Error processing program {programs_to_analyze[i].package_name}: {result}", file=sys.stderr)
                continue
            
            if result:
                repo_name = programs_to_analyze[i].repo_name
                if repo_name not in results_by_repo:
                    results_by_repo[repo_name] = []
                results_by_repo[repo_name].append(result)
    
    return results_by_repo


def create_markdown_report(repo_name: str, program_reports: List[Dict]) -> str:
    """Create a markdown report for all programs in a repository."""
    markdown_content = f"# {repo_name} - Solana Programs Analysis\n\n"
    
    for program in program_reports:
        cargo_path = program["cargo_toml_path"]
        report = program["report"]
        
        markdown_content += f"## {cargo_path}\n\n"
        markdown_content += f"{report}\n\n"
        markdown_content += "---\n\n"
    
    return markdown_content


async def main_async():
    """Async main function."""
    # Load environment variables from .env file
    load_dotenv()
    
    parser = argparse.ArgumentParser(
        description="Analyze Solana programs using LLM to generate comprehensive reports. "
    )
    parser.add_argument(
        "directory_path",
        help="Root directory path to scan for subdirectories"
    )
    parser.add_argument(
        "target_dependency",
        help="Name of the dependency to search for (e.g., 'anchor-lang')"
    )
    parser.add_argument(
        "system_prompt_file",
        help="Path to text file containing the system prompt for Solana development context"
    )
    parser.add_argument(
        "--exclude-dependency",
        help="Name of the dependency to exclude - skip Cargo files that contain this dependency"
    )
    parser.add_argument(
        "--max-concurrent",
        type=int,
        default=4,
        help="Maximum number of concurrent programs to process (default: 4)"
    )
    parser.add_argument(
        "--tokens-per-minute",
        type=int,
        default=80000,
        help="Maximum tokens per minute for API rate limiting (default: 80000)"
    )
    
    args = parser.parse_args()
    
    root_path = Path(args.directory_path)
    target_dependency = args.target_dependency
    system_prompt_path = Path(args.system_prompt_file)
    exclude_dependency = args.exclude_dependency
    max_concurrent = args.max_concurrent
    tokens_per_minute = args.tokens_per_minute
    
    # Validation
    if not root_path.exists():
        print(f"Error: Directory {root_path} does not exist", file=sys.stderr)
        sys.exit(1)
    
    if not root_path.is_dir():
        print(f"Error: {root_path} is not a directory", file=sys.stderr)
        sys.exit(1)
    
    if not system_prompt_path.exists():
        print(f"Error: System prompt file {system_prompt_path} does not exist", file=sys.stderr)
        sys.exit(1)
    
    # Read system prompt
    system_prompt = read_system_prompt(system_prompt_path)
    
    # Setup OpenAI client
    api_key = os.getenv("ANALYZE_REPO_LLM_API_KEY")
    if not api_key:
        print("Error: ANALYZE_REPO_LLM_API_KEY environment variable must be set", file=sys.stderr)
        sys.exit(1)

    base_url = os.getenv("ANALYZE_REPO_LLM_BASE_URL")
    if not base_url:
        print("Error: ANALYZE_REPO_LLM_BASE_URL environment variable must be set", file=sys.stderr)
        sys.exit(1)

    model = os.getenv("ANALYZE_REPO_LLM_MODEL")
    if not model:
        print("Error: ANALYZE_REPO_LLM_MODEL environment variable must be set", file=sys.stderr)
        sys.exit(1)
    
    client_kwargs = {"api_key": api_key, "base_url": base_url}
    client = OpenAI(**client_kwargs)
    
    # Initialize rate limiter
    rate_limiter = TokenRateLimiter(tokens_per_minute, model)
    
    # Find all programs to analyze upfront
    programs_to_analyze = find_programs_to_analyze(root_path, target_dependency, exclude_dependency)
    
    if not programs_to_analyze:
        print(f"No programs found with '{target_dependency}' dependency")
        if exclude_dependency:
            print(f"(excluding programs with '{exclude_dependency}' dependency)")
        sys.exit(0)
    
    # Show summary of what will be analyzed
    repo_counts = {}
    for program in programs_to_analyze:
        repo_counts[program.repo_name] = repo_counts.get(program.repo_name, 0) + 1
    
    print(f"\nFound {len(programs_to_analyze)} programs to analyze across {len(repo_counts)} repositories:")
    for repo_name, count in sorted(repo_counts.items()):
        print(f"  {repo_name}: {count} programs")
    
    print(f"\nTarget dependency: {target_dependency}")
    if exclude_dependency:
        print(f"Excluding dependency: {exclude_dependency}")
    print(f"Max concurrent: {max_concurrent}")
    print(f"Token rate limit: {tokens_per_minute:,} tokens/minute")
    
    # Process all programs with async progress tracking
    results_by_repo = await process_programs_with_progress_async(
        client, system_prompt, model, programs_to_analyze, root_path, max_concurrent, rate_limiter
    )
    
    # Create markdown reports for each repository
    total_analyzed = 0
    for repo_name, program_reports in results_by_repo.items():
        if program_reports:
            markdown_content = create_markdown_report(repo_name, program_reports)
            markdown_filename = f"{repo_name}.md"
            markdown_path = root_path / markdown_filename
            
            try:
                with open(markdown_path, 'w', encoding='utf-8') as f:
                    f.write(markdown_content)
                
                print(f"\nCreated: {markdown_path}")
                print(f"  Analyzed {len(program_reports)} programs")
                total_analyzed += len(program_reports)
                
            except Exception as e:
                print(f"Error writing {markdown_path}: {e}", file=sys.stderr)
    
    print(f"\nTotal programs successfully analyzed: {total_analyzed}")


def main():
    """Synchronous main function that runs the async version."""
    asyncio.run(main_async())


if __name__ == "__main__":
    main()
