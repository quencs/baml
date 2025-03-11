package baml_client

import (
	"context"

	baml "github.com/boundaryml/baml/go"
)

var bamlRuntime *baml.BamlRuntime

func init() {
	runtime, err := baml.CreateRuntime("./baml_src", map[string]string{
		"./baml_src/root.baml": `
		client<llm> Ollama {
			provider ollama
			options {
				model llama2
				max_tokens 100
			}
		}
		function TestOllama() -> string {
			client Ollama
			prompt #"
				Write a nice haiku about banks
			"#
		}
		`,
	}, map[string]string{})
	if err != nil {
		panic(err)
	}
	bamlRuntime = &runtime
}

type testOllama struct{}

var TestOllama = &testOllama{}

func (t *testOllama) Call(ctx context.Context) (string, error) {
	result, err := bamlRuntime.CallFunction(ctx, "TestOllama", []string{})
	if err != nil {
		return "", err
	}
	return result.Raw(), nil
}

func (t *testOllama) Stream(ctx context.Context) <-chan string {
	channel := make(chan string)
	resultChannel, err := bamlRuntime.CallFunctionStream(ctx, "TestOllama", []string{})
	if err != nil {
		close(channel)
		return channel
	}
	go func() {
		for {
			select {
			case <-ctx.Done():
				close(channel)
				return
			case result, ok := <-resultChannel:
				if !ok {
					close(channel)
					return
				}
				channel <- result.Raw()
			}
		}
	}()
	return channel
}

// b.ExtractResume(resume)
// b.ExtractResume.Call(resume)
// b.ExtractResume.CallSync(resume)
// b.ExtractResume.Stream(resume)
// b.ExtractResume.StreamSync(resume)
// b.ExtractResume.Request(resume)
// b.ExtractResume.Parse(resume)
// b.ExtractResume.ParseStream(resume)

// b.ExtractResume(resume).Call(resume)
// b.ExtractResume.Call(resume)
// b.ExtractResume.Stream(resume)
// b.ExtractResume.Request(resume)
// b.ExtractResume.Parse(resume)
// b.ExtractResume.ParseStream(resume)

// b.Call.ExtractResume(resume)
// b.Stream.ExtractResume(resume)

// b.ExtractResume(resume)
// b.ExtractResumeStream(resume)
