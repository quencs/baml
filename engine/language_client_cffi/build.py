#!/usr/bin/env python3
"""
Cross-Platform Build Script
===========================
A beautiful, feature-rich build script for compiling your project
across multiple target platforms.
"""

import argparse
from io import TextIOWrapper
import os
import platform
import shutil
import subprocess
import threading
import queue
import sys
import time
from dataclasses import dataclass
from enum import Enum
from typing import List, Optional, Dict


# ANSI Color Codes for beautiful terminal output
class Color:
    RESET = "\033[0m"
    BOLD = "\033[1m"
    UNDERLINE = "\033[4m"
    BLACK = "\033[30m"
    RED = "\033[31m"
    GREEN = "\033[32m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    WHITE = "\033[37m"
    BRIGHT_BLACK = "\033[90m"
    BRIGHT_RED = "\033[91m"
    BRIGHT_GREEN = "\033[92m"
    BRIGHT_YELLOW = "\033[93m"
    BRIGHT_BLUE = "\033[94m"
    BRIGHT_MAGENTA = "\033[95m"
    BRIGHT_CYAN = "\033[96m"
    BRIGHT_WHITE = "\033[97m"
    BG_BLACK = "\033[40m"
    BG_RED = "\033[41m"
    BG_GREEN = "\033[42m"
    BG_YELLOW = "\033[43m"
    BG_BLUE = "\033[44m"
    BG_MAGENTA = "\033[45m"
    BG_CYAN = "\033[46m"
    BG_WHITE = "\033[47m"

    @staticmethod
    def disable_if_not_supported():
        """Disable colors if terminal doesn't support them"""
        if (
            not sys.stdout.isatty()
            or platform.system() == "Windows"
            and not os.environ.get("TERM")
        ):
            for attr in dir(Color):
                if not attr.startswith("__") and isinstance(getattr(Color, attr), str):
                    setattr(Color, attr, "")


class TargetType(Enum):
    LINUX = "linux"
    MACOS = "darwin"
    WINDOWS = "windows"
    MUSL = "musl"


@dataclass
class Target:
    """Target platform configuration"""

    triple: str
    description: str
    type: TargetType

    @property
    def display_name(self) -> str:
        """Returns a nicely formatted display name"""
        return f"{self.description} ({self.triple})"

    @property
    def color(self) -> str:
        """Returns an appropriate color based on target type"""
        if self.type == TargetType.LINUX:
            return Color.YELLOW
        elif self.type == TargetType.MACOS:
            return Color.BRIGHT_CYAN
        elif self.type == TargetType.WINDOWS:
            return Color.BRIGHT_BLUE
        elif self.type == TargetType.MUSL:
            return Color.BRIGHT_GREEN
        return Color.WHITE


# Define available build targets
TARGETS = [
    Target("aarch64-unknown-linux-gnu", "Linux ARM64", TargetType.LINUX),
    Target("x86_64-unknown-linux-gnu", "Linux x86_64", TargetType.LINUX),
    Target("x86_64-unknown-linux-musl", "Linux MUSL x86_64", TargetType.MUSL),
    Target("x86_64-pc-windows-msvc", "Windows x86_64", TargetType.WINDOWS),
    Target("aarch64-pc-windows-msvc", "Windows ARM64", TargetType.WINDOWS),
    Target("x86_64-apple-darwin", "macOS x86_64", TargetType.MACOS),
    Target("aarch64-apple-darwin", "macOS ARM64", TargetType.MACOS),
]

# Default enabled targets
DEFAULT_ENABLED_TARGETS = [
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    # "x86_64-pc-windows-msvc",
    # "aarch64-pc-windows-msvc",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
]


class BuildEnvironment:
    """Information about the current build environment"""

    def __init__(self):
        self.system = platform.system()
        self.machine = platform.machine()
        self.release = platform.release()
        self.version = platform.version()
        self.processor = platform.processor() or self.machine

        # Normalize architecture names
        if self.machine == "arm64":
            self.machine = "aarch64"

    @property
    def display_info(self) -> str:
        """Returns formatted environment information"""
        return (
            f"{Color.BOLD}System:{Color.RESET} {self.system} {self.release}\n"
            f"{Color.BOLD}Architecture:{Color.RESET} {self.machine} ({self.processor})\n"
            f"{Color.BOLD}Version:{Color.RESET} {self.version}"
        )

    @property
    def is_windows(self) -> bool:
        return self.system == "Windows"

    @property
    def is_mac(self) -> bool:
        return self.system == "Darwin"

    @property
    def is_linux(self) -> bool:
        return self.system == "Linux"

    @property
    def is_musl(self) -> bool:
        if not self.is_linux:
            return False
        # Check if the system is using musl
        try:
            ldd_output = subprocess.run(
                ["ldd", "--version"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
            ).stderr
            return "musl" in ldd_output.lower()
        except:
            return False

    @property
    def is_x86_64(self) -> bool:
        return self.machine == "x86_64"

    @property
    def is_aarch64(self) -> bool:
        return self.machine == "aarch64"


class ProgressSpinner:
    """A spinner with streaming output to show progress during long operations"""

    FRAMES = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
    MAX_OUTPUT_LINES = 10  # Show last 10 lines of output

    def __init__(self, message: str):
        self.message = message
        self.frame_index = 0
        self.start_time = time.time()
        self.active = False
        self.output_lines: list[str] = []
        self.header_printed = False

    def start(self):
        """Start the spinner"""
        self.active = True
        self.start_time = time.time()
        self._update_frame()

    def update_output(self, line: str):
        """Update the streaming output with a new line"""
        if not line.strip():
            return

        # Truncate very long lines to prevent display issues
        max_line_length = 70
        if len(line) > max_line_length:
            line = line[:max_line_length] + "..."

        self.output_lines.append(line.rstrip())
        if len(self.output_lines) > self.MAX_OUTPUT_LINES:
            self.output_lines.pop(0)

        if self.active:
            self._redraw_full()

    def stop(self, success: bool = True, message: Optional[str] = None):
        """Stop the spinner with a success or error message"""
        self.active = False
        elapsed = time.time() - self.start_time

        # Clear all the output lines and the spinner
        if self.header_printed:
            # Calculate how many lines to move up
            displayed_lines = min(len(self.output_lines), self.MAX_OUTPUT_LINES)
            lines_to_clear = displayed_lines + 2 if displayed_lines > 0 else 1
            sys.stdout.write(f"\033[{lines_to_clear}A\033[J")
        else:
            sys.stdout.write("\r\033[K")

        if message:
            status_icon = (
                f"{Color.GREEN}✓{Color.RESET}"
                if success
                else f"{Color.RED}✗{Color.RESET}"
            )
            print(
                f"{status_icon} {message} {Color.BRIGHT_BLACK}({elapsed:.2f}s){Color.RESET}"
            )

    def _redraw_full(self):
        """Redraw the entire output area including spinner and output lines"""
        if not self.active:
            return

        # Move cursor back to the beginning if we've printed the header before
        if self.header_printed:
            num_lines = min(len(self.output_lines) + 1, self.MAX_OUTPUT_LINES + 1)
            sys.stdout.write(f"\033[{num_lines}A\r")

        # Draw the spinner line with full terminal width clearing
        frame = self.FRAMES[self.frame_index]
        sys.stdout.write(
            f"\r\033[K{Color.BRIGHT_CYAN}{frame}{Color.RESET} {self.message}\n"
        )

        # Draw the output box header if we have output
        if self.output_lines:
            sys.stdout.write(
                f"\033[K{Color.BRIGHT_BLACK}┌── Live Build Output ───────────────────────────{Color.RESET}\n"
            )

            # Print each output line with full line clearing
            for line in self.output_lines[-self.MAX_OUTPUT_LINES :]:
                sys.stdout.write(f"\033[K{Color.BRIGHT_BLACK}│{Color.RESET} {line}\n")

            # Print the bottom of the box if we have fewer than MAX_OUTPUT_LINES
            if len(self.output_lines) < self.MAX_OUTPUT_LINES:
                sys.stdout.write(
                    f"\033[K{Color.BRIGHT_BLACK}└───────────────────────────────────────────{Color.RESET}"
                )

        sys.stdout.flush()
        self.header_printed = True

    def _update_frame(self):
        """Update the spinner frame"""
        if not self.active:
            return

        self.frame_index = (self.frame_index + 1) % len(self.FRAMES)
        self._redraw_full()

        if self.active:
            # Schedule the next frame update
            from threading import Timer

            Timer(0.1, self._update_frame).start()


class Builder:
    """Main build coordinator"""

    def __init__(self, options: "BuildOptions"):
        self.options = options
        self.env = BuildEnvironment()
        self.results: Dict[str, bool] = {}

    def print_header(self):
        """Print a beautiful header for the build process"""
        header = f"""
{Color.BRIGHT_MAGENTA}╔{'═' * 60}╗
║{Color.BRIGHT_WHITE} ♦ Cross-Platform Build Tool {' ' * 34}║
{Color.BRIGHT_MAGENTA}╠{'═' * 60}╣{Color.RESET}
"""
        print(header)
        print(f"{Color.BOLD}Build Environment:{Color.RESET}")
        print(self.env.display_info)
        print(f"\n{Color.BOLD}Build Configuration:{Color.RESET}")
        print(f"  {Color.BOLD}Verbose:{Color.RESET} {self.options.verbose}")
        print(f"  {Color.BOLD}Debug Mode:{Color.RESET} {not self.options.release}")

        selected_targets = self.options.targets
        print(f"\n{Color.BOLD}Selected Targets:{Color.RESET}")
        for target in [t for t in TARGETS if t.triple in selected_targets]:
            print(f"  {target.color}● {target.display_name}{Color.RESET}")

        print(f"\n{Color.BRIGHT_MAGENTA}{'─' * 62}{Color.RESET}\n")

    def build_target(self, target: Target) -> bool:
        """Build for a specific target"""
        spinner = ProgressSpinner(
            f"Building for {target.color}{target.display_name}{Color.RESET}"
        )
        spinner.start()

        # Prepare environment variables
        env_vars = os.environ.copy()

        # Special handling for cross-compiling from Mac to Linux
        if self.env.is_mac and target.type == TargetType.LINUX:
            env_vars["CROSS_CONTAINER_OPTS"] = "--platform linux/amd64"

        # Add any user-specified environment variables
        for key, value in self.options.env_vars.items():
            env_vars[key] = value

        # Prepare command
        cmd = ["cross", "build"]
        if self.options.release:
            cmd.append("--release")
        if self.options.verbose:
            cmd.append("--verbose")
        cmd.extend(["--target", target.triple])

        # Add any extra build flags
        cmd.extend(self.options.extra_flags)

        try:
            # Create a queue for output lines
            output_queue: queue.Queue[str] = queue.Queue()

            # Helper function to read output and put in queue
            def read_output(stream: TextIOWrapper, prefix: str = ""):
                for line in iter(stream.readline, ""):
                    if prefix:
                        output_queue.put(f"{prefix} {line}")
                    else:
                        output_queue.put(line)

            # Use Popen to stream output in real-time
            with subprocess.Popen(
                cmd,
                env=env_vars,
                stdout=subprocess.PIPE if not self.options.verbose else None,
                stderr=subprocess.PIPE if not self.options.verbose else None,
                text=True,
                bufsize=1,  # Line buffered
                universal_newlines=True,
            ) as proc:
                # Set up threads to read stdout and stderr
                if not self.options.verbose and proc.stdout and proc.stderr:
                    stdout_thread = threading.Thread(
                        target=read_output, args=(proc.stdout, "")
                    )
                    stderr_thread = threading.Thread(
                        target=read_output,
                        args=(proc.stderr, f"{Color.RED}[ERROR]{Color.RESET}"),
                    )

                    stdout_thread.daemon = True
                    stderr_thread.daemon = True

                    stdout_thread.start()
                    stderr_thread.start()

                    # Process output queue and update spinner
                    while proc.poll() is None:
                        try:
                            # Get output line with a timeout to prevent blocking
                            line = output_queue.get(timeout=0.1)
                            spinner.update_output(line)
                            output_queue.task_done()
                        except queue.Empty:
                            # No output available, continue checking
                            continue

                    # Process any remaining output
                    while not output_queue.empty():
                        line = output_queue.get()
                        spinner.update_output(line)
                        output_queue.task_done()

                    # Wait for threads to finish
                    stdout_thread.join(1.0)
                    stderr_thread.join(1.0)
                else:
                    # Wait for process to complete if not capturing output
                    proc.wait()

                success = proc.returncode == 0

            # Store the result
            self.results[target.triple] = success

            # Show appropriate message
            if success:
                spinner.stop(
                    True,
                    f"Successfully built for {target.color}{target.display_name}{Color.RESET}",
                )
            else:
                spinner.stop(
                    False,
                    f"Failed to build for {target.color}{target.display_name}{Color.RESET}",
                )
                # No need to display output again since we streamed it live

            return success

        except Exception as e:
            spinner.stop(
                False,
                f"Error building for {target.color}{target.display_name}{Color.RESET}",
            )
            print(f"{Color.RED}Error: {str(e)}{Color.RESET}")
            self.results[target.triple] = False
            return False

    def build_all(self):
        """Build all selected targets"""
        self.print_header()

        start_time = time.time()
        if self.options.release:
            path = "release"
        else:
            path = "debug"
        for target in [t for t in TARGETS if t.triple in self.options.targets]:
            if self.build_target(target):
                match target.type:
                    case TargetType.MACOS:
                        extension = "dylib"
                    case TargetType.LINUX | TargetType.MUSL:
                        extension = "so"
                    case TargetType.WINDOWS:
                        extension = "dll"

                dir_path = os.path.dirname(__file__)
                # cp the target to the language_client_go/lib directory
                print(
                    f"{dir_path}/../target/{target.triple}/{path}/libbaml_cffi.{extension} -> {dir_path}/lib/libbaml_cffi-{target.triple}.{extension}"
                )
                shutil.copy(
                    f"{dir_path}/../target/{target.triple}/{path}/libbaml_cffi.{extension}",
                    f"{dir_path}/lib/libbaml_cffi-{target.triple}.{extension}",
                )
            else:
                if self.options.fail_fast:
                    print(
                        f"\n{Color.RED}Build failed. Stopping due to --fail-fast option.{Color.RESET}"
                    )
                    break

        elapsed = time.time() - start_time

        # Print summary
        success_count = sum(1 for success in self.results.values() if success)
        total_count = len(self.results)

        print(f"\n{Color.BRIGHT_MAGENTA}{'─' * 62}{Color.RESET}")

        if success_count == total_count:
            result_color = Color.GREEN
            result_text = "SUCCESS"
        elif success_count == 0:
            result_color = Color.RED
            result_text = "FAILED"
        else:
            result_color = Color.YELLOW
            result_text = "PARTIAL SUCCESS"

        print(
            f"\n{Color.BOLD}Build Summary:{Color.RESET} {result_color}{result_text}{Color.RESET}"
        )
        print(f"  {Color.BOLD}Total Time:{Color.RESET} {elapsed:.2f} seconds")
        print(
            f"  {Color.BOLD}Targets:{Color.RESET} {success_count}/{total_count} successful"
        )

        # Print individual results
        print(f"\n{Color.BOLD}Target Results:{Color.RESET}")
        for target in [t for t in TARGETS if t.triple in self.results]:
            success = self.results[target.triple]
            status_icon = (
                f"{Color.GREEN}✓{Color.RESET}"
                if success
                else f"{Color.RED}✗{Color.RESET}"
            )
            print(f"  {status_icon} {target.color}{target.display_name}{Color.RESET}")

        print(f"\n{Color.BRIGHT_MAGENTA}{'═' * 62}{Color.RESET}\n")

        # Return appropriate exit code
        return success_count == total_count


class BuildOptions:
    """Options for the build process"""

    def __init__(
        self,
        targets: Optional[List[str]] = None,
        release: bool = True,
        verbose: bool = False,
        fail_fast: bool = False,
        extra_flags: Optional[List[str]] = None,
        env_vars: Optional[Dict[str, str]] = None,
    ):
        self.targets = targets or DEFAULT_ENABLED_TARGETS
        self.release = release
        self.verbose = verbose
        self.fail_fast = fail_fast
        self.extra_flags = extra_flags or []
        self.env_vars = env_vars or {}


def parse_args() -> BuildOptions:
    """Parse command line arguments"""
    parser = argparse.ArgumentParser(
        description="Cross-platform build script",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter,
    )

    # Target options
    target_group = parser.add_argument_group("Target Options")
    target_group.add_argument(
        "--targets",
        nargs="*",
        default=DEFAULT_ENABLED_TARGETS,
        help="Specific targets to build",
    )
    target_group.add_argument(
        "--list-targets",
        action="store_true",
        help="List all available targets and exit",
    )

    # Build options
    build_group = parser.add_argument_group("Build Options")
    build_group.add_argument(
        "--debug",
        action="store_true",
        help="Build in debug mode (default is release mode)",
    )
    build_group.add_argument(
        "--verbose", action="store_true", help="Enable verbose output"
    )
    build_group.add_argument(
        "--fail-fast", action="store_true", help="Stop building after first failure"
    )
    build_group.add_argument(
        "--extra-flags", nargs="*", default=[], help="Extra flags to pass to cargo"
    )

    # Environment options
    env_group = parser.add_argument_group("Environment Options")
    env_group.add_argument(
        "--env",
        nargs="*",
        default=[],
        help="Additional environment variables in KEY=VALUE format",
    )

    args = parser.parse_args()

    # Handle --list-targets
    if args.list_targets:
        print(f"{Color.BOLD}Available Targets:{Color.RESET}")
        for target in TARGETS:
            enabled = "✓" if target.triple in DEFAULT_ENABLED_TARGETS else " "
            print(f"  [{enabled}] {target.color}{target.display_name}{Color.RESET}")
        sys.exit(0)

    # Parse environment variables
    env_vars = {}
    for env_var in args.env:
        if "=" in env_var:
            key, value = env_var.split("=", 1)
            env_vars[key] = value
        else:
            print(
                f"{Color.YELLOW}Warning: Ignoring malformed environment variable: {env_var}{Color.RESET}"
            )

    return BuildOptions(
        targets=args.targets,
        release=not args.debug,
        verbose=args.verbose,
        fail_fast=args.fail_fast,
        extra_flags=args.extra_flags,
        env_vars=env_vars,
    )


def main():
    """Main entry point"""
    # Initialize color support
    Color.disable_if_not_supported()

    # Parse command line arguments
    options = parse_args()

    # Run the build process
    builder = Builder(options)
    success = builder.build_all()

    # Return appropriate exit code
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print(f"\n\n{Color.YELLOW}Build interrupted by user.{Color.RESET}")
        sys.exit(130)
    except Exception as e:
        print(f"\n\n{Color.RED}Unhandled error: {str(e)}{Color.RESET}")
        if "--verbose" in sys.argv:
            import traceback

            traceback.print_exc()
        sys.exit(1)
