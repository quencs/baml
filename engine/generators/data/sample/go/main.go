package main

import (
	"context"
	"fmt"
	b "sample/baml_client"
)

func main() {
	// result, err := b.Foo(context.Background(), 8192)
	// if err != nil {
	// 	panic(err)
	// }
	// fmt.Println(result)

	channel := b.Stream.Foo(context.Background(), 8192)
	for result := range channel {
		if result.IsFinal {
			fmt.Println("final-----")
			fmt.Println(result.Final())
		} else {
			fmt.Println("stream-----")
			fmt.Println(result.Stream())
		}
	}
}
