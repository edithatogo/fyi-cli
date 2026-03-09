#!/usr/bin/env python3
"""Simple mutation testing script for Windows.

This script performs basic mutation testing by:
1. Finding all Python files in src/fyi_system
2. Introducing simple mutations (changing operators, values, etc.)
3. Running tests to see if mutations are caught
4. Reporting mutation score
"""
import os
import re
import subprocess
import sys
from pathlib import Path
from typing import List, Tuple

# Simple mutations to try
MUTATIONS = [
    # Boolean mutations
    (r'\bTrue\b', 'False', 'bool_true_to_false'),
    (r'\bFalse\b', 'True', 'bool_false_to_true'),
    # Comparison mutations
    (r'==', '!=', 'eq_to_neq'),
    (r'!=', '==', 'neq_to_eq'),
    (r'<', '<=', 'lt_to_lte'),
    (r'>', '>=', 'gt_to_gte'),
    # Arithmetic mutations
    (r'\+1', '-1', 'plus_one_to_minus_one'),
    (r'-1', '+1', 'minus_one_to_plus_one'),
    (r'\+ 1', '- 1', 'plus_space_one_to_minus'),
    # None mutations
    (r'\bis None\b', 'is not None', 'is_none_to_is_not_none'),
    (r'\bis not None\b', 'is None', 'is_not_none_to_is_none'),
    # Return mutations
    (r'return True', 'return False', 'return_true_to_false'),
    (r'return False', 'return True', 'return_false_to_true'),
    (r'return None', 'return "MUTATED"', 'return_none_to_string'),
]

def find_python_files(src_dir: Path) -> List[Path]:
    """Find all Python files in source directory."""
    return list(src_dir.glob('**/*.py'))

def read_file(path: Path) -> str:
    """Read file content."""
    return path.read_text(encoding='utf-8')

def write_file(path: Path, content: str):
    """Write file content."""
    path.write_text(content, encoding='utf-8')

def apply_mutation(content: str, pattern: str, replacement: str) -> Tuple[str, int]:
    """Apply a single mutation to content. Returns mutated content and count."""
    mutated, count = re.subn(pattern, replacement, content, count=1)
    return mutated, count

def run_tests() -> bool:
    """Run test suite. Returns True if tests pass."""
    # Exclude problematic tests and run core tests only for speed
    result = subprocess.run(
        ['python', '-m', 'pytest', 'tests/', 
         '--ignore=tests/test_scheduler_internals.py',
         '--ignore=tests/test_e2e_cli.py',
         '--ignore=tests/test_e2e_workflows.py',
         '--ignore=tests/test_api_contract.py',
         '-x', '-q', '--tb=no',
         '-k', 'not (SecureFileOperations or DataIntegrity)'],
        capture_output=True,
        text=True,
        timeout=300  # 5 minutes for full test suite
    )
    return result.returncode == 0

def mutate_and_test(file_path: Path, original_content: str, pattern: str, replacement: str, mutation_name: str) -> bool:
    """Apply mutation and run tests. Returns True if mutation was caught (tests failed)."""
    try:
        # Apply mutation
        mutated, count = apply_mutation(original_content, pattern, replacement)
        if count == 0:
            return False  # No mutation applied
        
        # Write mutated file
        write_file(file_path, mutated)
        
        # Run tests
        tests_passed = run_tests()
        
        # Restore original
        write_file(file_path, original_content)
        
        # If tests passed, mutation survived (bad)
        # If tests failed, mutation was caught (good)
        return not tests_passed
        
    except subprocess.TimeoutExpired:
        # Restore original on timeout
        write_file(file_path, original_content)
        return False
    except Exception as e:
        # Restore original on error
        write_file(file_path, original_content)
        return False

def main():
    """Run mutation testing."""
    src_dir = Path('src/fyi_system')
    test_dir = Path('tests')
    
    if not src_dir.exists():
        print(f"Error: Source directory {src_dir} not found")
        return 1
    
    if not test_dir.exists():
        print(f"Error: Test directory {test_dir} not found")
        return 1
    
    # First verify tests pass on original code
    print("Verifying tests pass on original code...")
    if not run_tests():
        print("ERROR: Tests fail on original code! Fix tests first.")
        return 1
    print("[OK] Original tests pass\n")
    
    # Find all Python files
    python_files = find_python_files(src_dir)
    print(f"Found {len(python_files)} Python files to mutate\n")
    
    total_mutations = 0
    caught_mutations = 0
    survived_mutations = 0
    
    # Try mutations on each file
    for file_path in python_files:
        print(f"Testing {file_path.name}...")
        original_content = read_file(file_path)
        
        for pattern, replacement, mutation_name in MUTATIONS:
            # Check if pattern exists in file
            if re.search(pattern, original_content):
                total_mutations += 1
                
                # Apply mutation and test
                caught = mutate_and_test(file_path, original_content, pattern, replacement, mutation_name)
                
                if caught:
                    caught_mutations += 1
                    print(f"  [CAUGHT] {mutation_name}: CAUGHT")
                else:
                    survived_mutations += 1
                    print(f"  [SURVIVED] {mutation_name}: SURVIVED")
        
        print()
    
    # Report results
    print("=" * 60)
    print("MUTATION TESTING RESULTS")
    print("=" * 60)
    print(f"Total mutations attempted: {total_mutations}")
    print(f"Mutations caught: {caught_mutations}")
    print(f"Mutations survived: {survived_mutations}")
    
    if total_mutations > 0:
        mutation_score = (caught_mutations / total_mutations) * 100
        print(f"\nMutation score: {mutation_score:.1f}%")
        
        if mutation_score >= 90:
            print("[EXCELLENT] Tests catch most mutations!")
        elif mutation_score >= 70:
            print("[GOOD] Tests catch many mutations, but room for improvement")
        else:
            print("[NEEDS IMPROVEMENT] Many mutations survived, tests need improvement")
    else:
        print("\nNo mutations could be applied. Code may be too simple or patterns need adjustment.")
    
    return 0

if __name__ == '__main__':
    sys.exit(main())
