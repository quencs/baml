package main

import (
	"context"
	"fmt"

	b "example.com/integ-tests/baml_client"
	"example.com/integ-tests/baml_client/types"
)

func main() {
	ctx := context.Background()
	// v, err := b.TestOllama(ctx)
	// if err != nil {
	// 	panic(err)
	// }
	// fmt.Println(*v)

	param := types.Union__List__string__string{}
	param.SetString("oranges")
	v2, err := b.AaaSamOutputFormat(ctx, &param)
	if err != nil {
		panic(err)
	}
	fmt.Println(*v2)

	v2, err = b.AaaSamOutputFormat(ctx, nil)
	if err != nil {
		panic(err)
	}
	fmt.Println(*v2)
}
