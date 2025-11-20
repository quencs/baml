#!/usr/bin/env -S uv run --script
# /// script
# dependencies = []
# ///
import sys
import re
import argparse
import subprocess
from pathlib import Path
from datetime import date

REPO_ROOT = Path(__file__).resolve().parents[2]
WOOL_DIR = REPO_ROOT / "wools"
DOCS_DIR = WOOL_DIR / "docs"
PROPOSALS_DIR = DOCS_DIR / "proposals"

TEMPLATE = """---
id: {wool_id}
title: "{title}"
shepherds: {author}
status: Draft
created: {created_date}
---

# {wool_id}: {title}

## Summary

A concise explanation (3–8 sentences) of what this proposal does and what it enables for BAML users.

## Motivation

Why should this exist in BAML? What problems does it solve?

## Proposed Design

Describe the design in enough detail that another contributor could implement it.

### Syntax

```baml
// Example code
```

### Semantics

Explain how the compiler/runtime handles this.

### Backwards Compatibility

Does this break existing code?

## Alternatives Considered

What other approaches were considered and why were they rejected?
"""


def get_git_author():
    try:
        # Get name
        name = subprocess.check_output(["git", "config", "user.name"], text=True).strip()
        email = subprocess.check_output(["git", "config", "user.email"], text=True).strip()
        
        if not name:
            return "<Author>"
        
        if email:
            return f"{name} <{email}>"
        return name
    except Exception:
        return "<Author>"


def get_next_wool_number():
    if not PROPOSALS_DIR.exists():
        PROPOSALS_DIR.mkdir(parents=True)
        
    # Find all folders matching WOOL-* in proposals dir
    wool_folders = [d for d in PROPOSALS_DIR.glob("WOOL-*") if d.is_dir()]
    # Also check for standalone files just in case, to avoid collision (legacy support)
    wool_files = list(PROPOSALS_DIR.glob("WOOL-*.md"))
    
    max_num = -1
    
    for path in wool_folders + wool_files:
        m = re.search(r"WOOL-(\d+)", path.name)
        if m:
            num = int(m.group(1))
            if num > max_num:
                max_num = num
    
    return max_num + 1 if max_num >= 0 else 1


def slugify(text):
    text = text.lower()
    text = re.sub(r"[^a-z0-9]+", "-", text)
    return text.strip("-")


def main():
    parser = argparse.ArgumentParser(description="Create a new WOOL proposal.")
    parser.add_argument("title", help="The title of the feature proposal")
    parser.add_argument("--author", help="Author name(s)", default=None)
    args = parser.parse_args()

    title = args.title
    author = args.author or get_git_author()

    next_num = get_next_wool_number()
    wool_id = f"WOOL-{next_num:03d}"
    slug = slugify(title)
    
    # Create directory: wools/proposals/WOOL-XXX-title
    folder_name = f"{wool_id}-{slug}"
    folder_path = PROPOSALS_DIR / folder_name
    folder_path.mkdir(exist_ok=True, parents=True)
    
    # Create README.md inside the folder
    filepath = folder_path / "README.md"

    content = TEMPLATE.format(
        wool_id=wool_id,
        title=title,
        author=author,
        created_date=date.today().strftime("%Y-%m-%d"),
    )

    filepath.write_text(content, encoding="utf-8")
    print(f"Created new WOOL: {filepath}")


if __name__ == "__main__":
    main()

