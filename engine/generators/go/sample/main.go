package main

import (
	"fmt"
)

func main() {
	fmt.Println("Hello, World!")
}

type A struct {
}

func return_map(x int) map[string]int {
	return map[string]int{
		"x": x,
	}
}

type Union3AorBorC struct {
}
type Checked[T any] struct {
}

type Foo = Checked[*Union3AorBorC]
