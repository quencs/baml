"""
Test only the pickle functionality without calling BAML functions.
This isolates the pickle implementation from runtime issues.
"""

from multiprocessing import Process
from baml_client import b
from baml_client.sync_client import b as sync_b
import pickle


def test_runtime_pickle():
    """Test that the underlying runtime can be pickled."""
    try:
        print("Testing runtime pickle...")
        runtime = b._BamlAsyncClient__runtime  # Access the runtime directly
        pickled_runtime = pickle.dumps(runtime)
        unpickled_runtime = pickle.loads(pickled_runtime)
        print("✅ Runtime pickle/unpickle works!")
        return True
    except Exception as e:
        print(f"❌ Runtime pickle failed: {e}")
        return False


def test_async_client_pickle():
    """Test async client pickle (expected to fail)."""
    try:
        print("Testing async client pickle...")
        pickled_client = pickle.dumps(b)
        print("✅ Async client pickle works!")
        return True
    except Exception as e:
        print(f"❌ Async client pickle failed: {e}")
        print("   (This is expected due to ContextVar)")
        return False


def test_sync_client_pickle():
    """Test sync client pickle."""
    try:
        print("Testing sync client pickle...")
        pickled_client = pickle.dumps(sync_b)
        unpickled_client = pickle.loads(pickled_client)
        print("✅ Sync client pickle works!")
        return True
    except Exception as e:
        print(f"❌ Sync client pickle failed: {e}")
        return False


def worker():
    """Worker function that only tests pickle, no BAML calls."""
    print("Worker process started...")
    
    # Test 1: Runtime pickle
    runtime_result = test_runtime_pickle()
    
    # Test 2: Async client pickle
    async_result = test_async_client_pickle()
    
    # Test 3: Sync client pickle  
    sync_result = test_sync_client_pickle()
    
    print(f"Results - Runtime: {'✅' if runtime_result else '❌'}, "
          f"Async: {'✅' if async_result else '❌'}, "
          f"Sync: {'✅' if sync_result else '❌'}")


if __name__ == "__main__":
    print("=" * 60)
    print("BAML Pickle-Only Test (No Function Calls)")
    print("=" * 60)
    
    # Test in main process first
    print("\n1. Testing in main process:")
    worker()
    
    # Test in separate process
    print("\n2. Testing in separate process:")
    process = Process(target=worker)
    process.start()
    process.join()
    
    print(f"\nProcess exit code: {process.exitcode}")
    if process.exitcode == 0:
        print("✅ Multiprocessing test passed!")
    else:
        print("❌ Multiprocessing test failed!")
    
    print("=" * 60)