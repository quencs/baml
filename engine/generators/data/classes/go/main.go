package main

import (
	b "classes/baml_client"
	"context"
	"encoding/json"
	"fmt"
)

func main() {
	channel := b.Stream.MakeSimpleClass(context.Background())
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
		}
	}
}
