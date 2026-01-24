# Testing BAML Fork Safety

## Prerequisites

1. Redis server running
2. Rust extension built

## Build the Rust Extension

```bash
cd /Users/aaronvillalpando/Projects/baml/integ-tests/python-multiprocessing
uv run maturin develop --uv --manifest-path /Users/aaronvillalpando/Projects/baml/engine/language_client_python/Cargo.toml
```

## Start Redis (if not running)

```bash
redis-server
```

Verify:
```bash
redis-cli ping
```

## Test Routine

### Step 1: Start RQ Worker

```bash
cd /Users/aaronvillalpando/Projects/baml/integ-tests/python-multiprocessing
pkill -f "rqworker" 2>/dev/null; sleep 1
PYTHONPATH=. OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES uv run python -m app.manage rqworker async
```

### Step 2: Run Test (Separate Terminal)

```bash
cd /Users/aaronvillalpando/Projects/baml/integ-tests/python-multiprocessing
PYTHONPATH=. OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES uv run python -c "
from app.manage import test_async_worker
test_async_worker()
"
```

### Expected Output

Success looks like:
```
Test 2: Async in RQ worker...
Job ID: <uuid>
Result: None
```

With debug logging enabled, you should see:
```
[get_tokio_singleton] Fork detected! previous_pid=XXX, current_pid=YYY, marking as forked
[get_llm_provider_impl] was_forked()=true, PID=YYY
[get_llm_provider_impl] Forked child detected, skipping cache...
```

Failure (SIGSEGV) looks like:
```
Test 2: Async in RQ worker...
Job ID: <uuid>
Failed: Work-horse terminated unexpectedly; waitpid returned 11 (signal 11);
```

## Run Multiple Tests

```bash
for i in 1 2 3 4 5; do
  PYTHONPATH=. OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES uv run python -c "
from app.manage import test_async_worker
test_async_worker()
" 2>&1 | grep -E "(Result:|Failed:)"
done
```

## Environment Variables

- `OBJC_DISABLE_INITIALIZE_FORK_SAFETY=YES` - Required on macOS for fork safety
- `PYTHONPATH=.` - Required for local module imports
- `RUST_LOG=debug` - Optional: enables Rust debug logging
- `RUST_BACKTRACE=1` - Optional: enables backtraces on crash

## Cleanup

```bash
pkill -f "rqworker"
redis-cli shutdown
```

## Troubleshooting

### SIGSEGV in Forked Child

If you see `signal 11` crashes, the issue is likely:
1. DashMap access in forked child (fixed by `was_forked()` check)
2. Tokio runtime corruption (fixed by `fork_safe_future_into_py`)

Check debug output to see which component is failing.

### Job Hangs

If jobs hang, check:
1. Redis is running: `redis-cli ping`
2. Worker is listening: Look for `*** Listening on async...`
3. No Python import errors in worker output
