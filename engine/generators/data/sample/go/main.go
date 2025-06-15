package main

import (
	"context"
	"encoding/json"
	"fmt"
	b "sample/baml_client"
)

func main() {
	// result, err := b.Foo(context.Background(), 8192)
	// if err != nil {
	// 	panic(err)
	// }
	// fmt.Println(result)

	channel, err := b.Stream.Foo(context.Background(), 8192)
	if err != nil {
		panic(err)
	}
	for result := range channel {
		if result.IsFinal {
			fmt.Println("final-----")
			str, err := json.Marshal(result.Final())
			if err != nil {
				panic(err)
			}
			fmt.Println(string(str))
		} else {
			fmt.Println("stream-----")
			str, err := json.Marshal(result.Stream())
			if err != nil {
				panic(err)
			}
			fmt.Println(string(str))
		}
	}
}
