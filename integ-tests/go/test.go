package main

import (
	"context"
	"fmt"

	b "example.com/integ-tests/baml_client"
)

func main() {
	ctx := context.Background()
	// v, err := b.TestOllama(ctx)
	// if err != nil {
	// 	panic(err)
	// }
	// fmt.Println(*v)

	v2, err := b.AaaSamOutputFormat(ctx, "apple pie")
	if err != nil {
		panic(err)
	}
	fmt.Println(*v2)
}
