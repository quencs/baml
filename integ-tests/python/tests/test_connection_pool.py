# Tests to repro this issue: https://github.com/BoundaryML/baml/issues/2594
#
# The code here is not able to reproduce the issue, it suggests that concurrency
# actually works as expected. The idea is:
#
# 1. Start the Node server in concurrent_server.js that receives a --latency
#    flag and responds to any incoming request in that amount of time.
#
# 2. Send 30 requests concurrently from the Python client. If there was a
#    problem with the client, the total duration to get all the responses should
#    be much longer than the latency of a single response.
#
# That should prove that concurrency either works or does not.
#
# According to the Github issue, the first 6 requests run sequentially, so the
# total duration of the asyncio.gather() call should be at least 6 times the
# latency of a single response. But so far no luck in reproducing the bug.

import asyncio
import contextlib
import os
import pathlib
import shutil
import socket
import time

from baml_py import ClientRegistry
import pytest
import pytest_asyncio

from baml_client import b


# These are hardcoded in the Baml client. Can't be fully dynamic because the
# bug is present when URLs are written as strings instead of using the env var.
HOST = "127.0.0.1"
PORT = 9876


# For dynamic clients we can use ports not hardcoded in the Baml client.
def find_free_port():
    with contextlib.closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def port_is_free(host: str, port: int) -> bool:
    with contextlib.closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.settimeout(0.25)
        return s.connect_ex((host, port)) != 0



async def wait_for_port(host: str, port: int, timeout_s: float = 15.0):
    deadline = asyncio.get_running_loop().time() + timeout_s
    while True:
        try:
            reader, writer = await asyncio.open_connection(host, port)
            writer.close()
            try:
                await writer.wait_closed()
            except Exception:
                pass
            return
        except Exception:
            if asyncio.get_running_loop().time() > deadline:
                raise RuntimeError(f"Port {host}:{port} did not open in time")
            await asyncio.sleep(0.05)


async def http_health(host: str, port: int, timeout_s: float = 2.0):
    deadline = asyncio.get_running_loop().time() + timeout_s

    request = (
        "GET /health HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        "Connection: close\r\n"
        "\r\n"
    )

    while True:
        try:
            reader, writer = await asyncio.open_connection(host, port)
            writer.write(request.encode("ascii"))
            await writer.drain()
            data = await reader.read(-1)
            writer.close()
            try:
                await writer.wait_closed()
            except Exception:
                pass
            if b" 200 " in data:
                return True
        except Exception:
            pass

        if asyncio.get_running_loop().time() > deadline:
            return False
        await asyncio.sleep(0.05)


async def terminate_process(proc: asyncio.subprocess.Process):
    if proc.returncode is None:
        try:
            proc.terminate()
        except ProcessLookupError:
            return
        try:
            await asyncio.wait_for(proc.wait(), timeout=3)
        except asyncio.TimeoutError:
            try:
                proc.kill()
            except ProcessLookupError:
                pass


async def read_stdout(proc: asyncio.subprocess.Process, buf: list[str]):
    if not proc.stdout:
        return
    try:
        while True:
            line = await proc.stdout.readline()
            if not line:
                break
            try:
                buf.append(line.decode("utf-8", errors="ignore"))
            except Exception:
                buf.append(repr(line))
    except Exception:
        pass


@contextlib.asynccontextmanager
async def start_concurrency_test_server(latency: int):
    server_js_path = pathlib.Path(__file__).parent.parent.parent / "common" / "concurrent_server.js"
    if not server_js_path.exists():
        raise FileNotFoundError(f"Server script not found: {server_js_path}")

    node_bin = shutil.which("node") or shutil.which("nodejs")
    if not node_bin:
        raise RuntimeError("Cannot find 'node' or 'nodejs' on PATH")

    if not port_is_free(HOST, PORT):
        raise RuntimeError(f"Concurrency test expects Port {HOST}:{PORT} to be free but it is not")

    # In case we need additional logic.
    host = HOST
    port = PORT

    cmd = [node_bin, str(server_js_path), "--host", host, "--port", str(port), "--latency", str(latency)]

    base_url = f"http://{host}:{port}/v1/"

    os.environ["OPENAI_CONCURRENCY_TEST_BASE_URL"] = base_url
    env = os.environ.copy()

    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
        cwd=os.getcwd(),
        env=env,
    )

    log_buf: list[str] = []
    proc_stdout_task = asyncio.create_task(read_stdout(proc, log_buf))

    try:
        await wait_for_port(host, port, timeout_s=15.0)
        await http_health(host, port, timeout_s=2.0)
    except Exception as e:
        await terminate_process(proc)

        try:
            await asyncio.wait_for(proc_stdout_task, timeout=0.3)
        except Exception:
            pass

        logs = "".join(log_buf)

        raise RuntimeError(f"Failed to start Node server: {e}\n--- server output ---\n{logs}")

    try:
        yield base_url
    finally:
        await terminate_process(proc)

        try:
            await asyncio.wait_for(proc_stdout_task, timeout=0.3)
        except Exception:
            pass

        logs = "".join(log_buf)
        print(f"--- Concurrency Test Server Output ---\n{logs}")

        try:
            await asyncio.wait_for(proc_stdout_task, timeout=0.5)
        except Exception:
            pass


@pytest_asyncio.fixture(scope="module")
async def concurrency_server_url():
    async with start_concurrency_test_server(LATENCY_MS) as base_url:
        yield base_url


# Times the server takes to process one request.
LATENCY_MS = 500

# How many requests to make.
NUM_REQUESTS = 30

# Allow some extra time per request for scheduling / OS overhead.
ALLOWED_DEVIATION_MS = 3 * NUM_REQUESTS

# Expected duration in milliseconds of all concurrent requests.
EXPECTED_DURATION_MS = LATENCY_MS + ALLOWED_DEVIATION_MS


async def assert_completes_in_time(baml_requests: list[asyncio.Future]):
    start_time = time.perf_counter()
    timeout_s = max(5.0, (EXPECTED_DURATION_MS / 1000.0) + 2.0)

    results = await asyncio.wait_for(asyncio.gather(*baml_requests), timeout=timeout_s)

    duration_ms = (time.perf_counter() - start_time) * 1000.0

    assert len(results) == NUM_REQUESTS

    assert duration_ms <= EXPECTED_DURATION_MS, (
        f"Expected duration <= {EXPECTED_DURATION_MS} ms but got {duration_ms:.2f} ms; "
        f"requests may not be running concurrently."
    )


@pytest.mark.asyncio
async def test_openai_concurrency_client_hardcoded_base_url(concurrency_server_url: str):
    requests = [b.TestOpenAIConcurrencyClientHardocodedBaseUrl("test") for _ in range(NUM_REQUESTS)]

    await assert_completes_in_time(requests)


@pytest.mark.asyncio
async def test_openai_concurrency_client_env_var_base_url(concurrency_server_url: str):
    requests = [b.TestOpenAIConcurrencyClientEnvBaseUrl("test") for _ in range(NUM_REQUESTS)]

    await assert_completes_in_time(requests)


@pytest.mark.asyncio
async def test_openai_concurrency_client_registry(concurrency_server_url: str):
    cr = ClientRegistry()
    cr.add_llm_client("ConcurrencyTestClient", "openai-generic", {
        "model": "concurrency-test",
        "base_url": concurrency_server_url,
    })
    cr.set_primary("ConcurrencyTestClient")

    requests = [b.TestOpenAI("test", {"client_registry": cr}) for _ in range(NUM_REQUESTS)]

    await assert_completes_in_time(requests)