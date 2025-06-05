from baml_client import b


def main():
    result = b.Foo(8192)
    print(result)

    channel = b.stream.Foo(8192)
    for result in channel:
        print(result)


if __name__ == "__main__":
    main()
