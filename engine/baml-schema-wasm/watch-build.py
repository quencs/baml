#!/usr/bin/env -S uv run --quiet --script
# /// script
# requires-python = ">=3.11"
# ///
"""
Hot-reload build script for WASM development.
Called by bacon when Rust source files change.
"""

import fcntl
import os
import subprocess
import sys
import signal
from pathlib import Path

# Configuration
SCRIPT_DIR = Path(__file__).parent
LOCK_FILE = SCRIPT_DIR / "web" / ".wasm-build.lock"
STATUS_FILE = SCRIPT_DIR / "web" / ".wasm-build-status"
OUTPUT_FILE = SCRIPT_DIR / "web" / ".wasm-build-output.tmp"

# Global state for cleanup
lock_fd = None
should_cleanup = True


def write_status(status: str):
    """Write status and flush immediately."""
    STATUS_FILE.write_text(status)
    # Force filesystem sync
    os.sync()


def cleanup(signum=None, frame=None):
    """Clean up resources on exit."""
    global should_cleanup

    if not should_cleanup:
        return

    print("Cleaning up...", file=sys.stderr)

    # If interrupted while refreshing, mark as cancelled
    if STATUS_FILE.exists() and STATUS_FILE.read_text().strip() == "refreshing":
        write_status("cancelled")
        print("Status: Build cancelled", file=sys.stderr)

    # Remove temp files
    if OUTPUT_FILE.exists():
        OUTPUT_FILE.unlink()

    # Release lock (will happen automatically, but being explicit)
    if lock_fd is not None:
        try:
            fcntl.flock(lock_fd, fcntl.LOCK_UN)
            os.close(lock_fd)
        except:
            pass

    print("Cleanup complete", file=sys.stderr)
    should_cleanup = False


def main():
    global lock_fd

    # Change to script directory
    os.chdir(SCRIPT_DIR)

    # Set up signal handlers
    signal.signal(signal.SIGINT, cleanup)
    signal.signal(signal.SIGTERM, cleanup)

    # Create lock file if it doesn't exist
    LOCK_FILE.parent.mkdir(parents=True, exist_ok=True)
    LOCK_FILE.touch()

    # Try to acquire exclusive lock (non-blocking)
    try:
        lock_fd = os.open(str(LOCK_FILE), os.O_RDWR | os.O_CREAT)
        fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    except BlockingIOError:
        print("Another build is in progress, skipping...", file=sys.stderr)
        return 0

    print("Lock acquired, running build...", file=sys.stderr)

    # Signal that build is starting
    write_status("refreshing")
    print("Status: Build starting (refreshing)", file=sys.stderr)

    # Run wasm-pack build
    # Note: Using --release even for dev because --dev bundles are too slow
    cmd = [
        "wasm-pack",
        "build",
        "./",
        "--target",
        "bundler",
        "--out-dir",
        "./web/dist",
        # cant use --dev until we solve this issue: https://github.com/wasm-bindgen/wasm-bindgen/issues/1563
        "--dev",
    ]

    # Run process and capture output while displaying it
    with open(OUTPUT_FILE, "w") as output_f:
        process = subprocess.Popen(
            cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, bufsize=1
        )

        # Stream output to both stdout and file
        for line in process.stdout:
            print(line, end="")
            output_f.write(line)

        process.wait()
        exit_code = process.returncode

    print(f"Build exit code: {exit_code}", file=sys.stderr)

    # Write status based on exit code
    if exit_code == 0:
        write_status("success")
        print("=" * 47, file=sys.stderr)
        print("Status: Build succeeded (success written to file)", file=sys.stderr)
        print("=" * 47, file=sys.stderr)

        # Verify
        verify_status = STATUS_FILE.read_text().strip()
        print(f"Verified: status file contains: '{verify_status}'", file=sys.stderr)
    else:
        # Build failed - save output for Vite error overlay
        error_output = OUTPUT_FILE.read_text()
        write_status(error_output)
        print("=" * 47, file=sys.stderr)
        print("Status: Build failed (errors written)", file=sys.stderr)
        print("=" * 47, file=sys.stderr)

    # Clean up
    cleanup()

    return exit_code


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        cleanup()
        sys.exit(1)
