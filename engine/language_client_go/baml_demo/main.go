package main

import (
	b "baml_demo/baml_client"
	"context"
	"fmt"
)

func main() {
	ctx := context.Background()
	fmt.Println("Calling TestOllama")
	fmt.Println(b.TestOllama.Call(ctx))

	fmt.Println("Streaming TestOllama")
	count := 0

	for result := range b.TestOllama.Stream(ctx) {
		fmt.Println("--------------------------------")
		fmt.Println("Count:", count)
		fmt.Println(result)
	}
}
