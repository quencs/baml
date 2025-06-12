from multiprocessing import Process
from baml_client import b


def run():
    result = b.ExtractResume("test", None)
    print(f"Result: {result}")


if __name__ == "__main__":
    print("Testing multiprocessing with baml_client.b...")
    current = Process(target=run)
    current.start()
    current.join()
    print("Test completed")