package main

import (
	"context"
	"fmt"

	b "example.com/integ-tests/baml_client"
	baml "github.com/boundaryml/baml/engine/language_client_go/pkg"
)

func main() {
	ctx := context.Background()

	collector := baml.NewCollector()

	v2, err := b.AaaSamOutputFormat(ctx, "oranges", b.WithCollector(collector))
	if err != nil {
		panic(err)
	}
	fmt.Println(*v2)

	usage, err := collector.Usage()
	if err != nil {
		panic(err)
	}
	input_tokens, err := usage.InputTokens()
	if err != nil {
		panic(err)
	}
	output_tokens, err := usage.OutputTokens()
	if err != nil {
		panic(err)
	}
	fmt.Printf("input_tokens: %d\n", input_tokens)
	fmt.Printf("output_tokens: %d\n", output_tokens)

	last, err := collector.Last()
	if err != nil {
		panic(err)
	}
	usage, err = last.Usage()
	if err != nil {
		panic(err)
	}
	last_input_tokens, err := usage.InputTokens()
	if err != nil {
		panic(err)
	}
	last_output_tokens, err := usage.OutputTokens()
	if err != nil {
		panic(err)
	}

	fmt.Printf("last input tokens: %d\n", last_input_tokens)
	fmt.Printf("last output tokens: %d\n", last_output_tokens)

	id, err := last.Id()
	if err != nil {
		panic(err)
	}
	fmt.Printf("last id: %s\n", id)

	functionName, err := last.FunctionName()
	if err != nil {
		panic(err)
	}
	fmt.Printf("last function name: %s\n", functionName)

	timing, err := last.Timing()
	if err != nil {
		panic(err)
	}
	startTimeUtcMs, err := timing.StartTimeUtcMs()
	if err != nil {
		panic(err)
	}
	durationMs, err := timing.DurationMs()
	if err != nil {
		panic(err)
	}

	fmt.Printf("last start time utc ms: %d\n", startTimeUtcMs)
	fmt.Printf("last duration ms: %d\n", durationMs)

	rawLlmResponse, err := last.RawLlmResponse()
	if err != nil {
		panic(err)
	}
	fmt.Printf("last raw llm response: %s\n", rawLlmResponse)

	calls, err := last.Calls()
	if err != nil {
		panic(err)
	}
	for _, call := range calls {
		clientName, err := call.ClientName()
		if err != nil {
			panic(err)
		}
		fmt.Printf("call client name: %s\n", clientName)
		provider, err := call.Provider()
		if err != nil {
			panic(err)
		}
		fmt.Printf("call provider: %s\n", provider)
		timing, err := call.Timing()
		if err != nil {
			panic(err)
		}
		startTimeUtcMs, err := timing.StartTimeUtcMs()
		if err != nil {
			panic(err)
		}
		fmt.Printf("call start time utc ms: %d\n", startTimeUtcMs)
		durationMs, err := timing.DurationMs()
		if err != nil {
			panic(err)
		}
		fmt.Printf("call duration ms: %d\n", durationMs)
		usage, err := call.Usage()
		if err != nil {
			panic(err)
		}
		inputTokens, err := usage.InputTokens()
		if err != nil {
			panic(err)
		}
		fmt.Printf("call input tokens: %d\n", inputTokens)
		outputTokens, err := usage.OutputTokens()
		if err != nil {
			panic(err)
		}
		fmt.Printf("call output tokens: %d\n", outputTokens)
		selected, err := call.Selected()
		if err != nil {
			panic(err)
		}
		fmt.Printf("call selected: %t\n", selected)
	}

	// v2, err = b.AaaSamOutputFormat(ctx, "pineapple")
	// if err != nil {
	// 	panic(err)
	// }
	// fmt.Println(*v2)

	// stream := b.Stream.AaaSamOutputFormat(ctx, "pineapple")
	// for chunk := range stream {
	// 	fmt.Println(chunk)
	// }
}
