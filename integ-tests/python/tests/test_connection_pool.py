import asyncio
import contextlib
import os
import pathlib
import socket
import time
import shutil

from baml_py import ClientRegistry
import pytest
from baml_client import b


def find_free_port():
    with contextlib.closing(socket.socket(socket.AF_INET, socket.SOCK_STREAM)) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


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


async def try_http_health(host: str, port: int, timeout_s: float = 2.0):
    deadline = asyncio.get_running_loop().time() + timeout_s
    request = (
        "GET /health HTTP/1.1\r\n"
        f"Host: {host}:{port}\r\n"
        "Connection: close\r\n"
        "\r\n"
    ).encode("ascii")

    while True:
        try:
            reader, writer = await asyncio.open_connection(host, port)
            writer.write(request)
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


async def pump_stdout(proc: asyncio.subprocess.Process, buf: list[str]):
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
async def start_openai_generic_server(latency: int):
    server_js_path = pathlib.Path(__file__).parent.parent.parent / "common" / "concurrent_server.js"
    if not server_js_path.exists():
        raise FileNotFoundError(f"Server script not found: {server_js_path}")

    node_bin = shutil.which("node") or shutil.which("nodejs")
    if not node_bin:
        raise RuntimeError("Cannot find 'node' or 'nodejs' on PATH")

    host = "127.0.0.1"
    port = find_free_port()

    cmd = [node_bin, str(server_js_path), "--host", host, "--port", str(port), "--latency", str(latency)]
    env = os.environ.copy()

    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
        cwd=os.getcwd(),
        env=env,
    )

    log_buf: list[str] = []
    pump_task = asyncio.create_task(pump_stdout(proc, log_buf))

    base_url = f"http://{host}:{port}"

    try:
        await wait_for_port(host, port, timeout_s=15.0)
        await try_http_health(host, port, timeout_s=2.0)
    except Exception as e:
        await terminate_process(proc)

        try:
            await asyncio.wait_for(pump_task, timeout=0.3)
        except Exception:
            pass

        logs = "".join(log_buf)

        raise RuntimeError(f"Failed to start Node server: {e}\n--- server output ---\n{logs}")

    try:
        yield f"{base_url}/v1"
    finally:
        await terminate_process(proc)

        try:
            await asyncio.wait_for(pump_task, timeout=0.3)
        except Exception:
            pass

        logs = "".join(log_buf)
        print(f"--- Concurrency Test Server Output ---\n{logs}")

        try:
            await asyncio.wait_for(pump_task, timeout=0.5)
        except Exception:
            pass


@pytest.mark.asyncio
async def test_connection_pool_concurrency():
    # How many requests to make.
    num_requests = 20

    # How long the server takes to process one request.
    latency_ms = 500

    # Allow some extra time per request (scheduling overhead).
    allowed_deviation_ms = 3 * num_requests

    # If the requests are running concurrently, they should all complete within
    # the request latency plus some extra processing time. But if they run
    # sequentially in batches as the bug report suggests, they will take much
    # longer:
    #
    # https://github.com/BoundaryML/baml/issues/2594
    expected_duration_ms = latency_ms + allowed_deviation_ms

    async with start_openai_generic_server(latency_ms) as base_url:
        cr = ClientRegistry()
        cr.add_llm_client("ConcurrencyTestClient", "openai-generic", {
            "model": "concurrency-test",
            "base_url": base_url,
        })
        cr.set_primary("ConcurrencyTestClient")

        coros = [b.TestOpenAI("test", {"client_registry": cr}) for _ in range(num_requests)]

        start_time = time.perf_counter()
        timeout_s = max(5.0, (expected_duration_ms / 1000.0) + 2.0)

        results = await asyncio.wait_for(asyncio.gather(*coros), timeout=timeout_s)

        duration_ms = (time.perf_counter() - start_time) * 1000.0

        assert len(results) == num_requests
        assert duration_ms <= expected_duration_ms, (
            f"Expected duration <= {expected_duration_ms} ms but got {duration_ms:.2f} ms; "
            "requests may not be running concurrently."
        )
