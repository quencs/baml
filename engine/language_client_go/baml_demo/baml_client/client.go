package baml_client

import (
	"context"
	"encoding/json"
	"fmt"

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

		function TestOllama() -> string | int {
			client Ollama
			prompt #"
				Write a nice haiku about banks
			"#
		}

		type Result = string | int

		function TestOllama2() -> Result | null {
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

type Result struct {
	union_type string  `json:"baml_type" enum:"string,int,null"`
	String     *string `json:"value_string omitempty"`
	Int        *int    `json:"value_int omitempty"`
	// how do i restrict int to specific values (1,2,3) -> custom marshaler
	// value_must_be_int int `json:"value_must_be_int"`
}

// constructor
func (r *Result) IsString() bool {
	return r.union_type == "string"
}

func (r *Result) IsInt() bool {
	return r.union_type == "int"
}

func ResultFromString(value string) *Result {
	return &Result{
		union_type: "string",
		String:     &value,
	}
}

func ResultFromInt(value int) *Result {
	return &Result{
		union_type: "int",
		Int:        &value,
	}
}

type Result2 struct {
	union_type string  `json:"baml_type" enum:"string,int,null"`
	String     *string `json:"value_string omitempty"`
	Int        *int    `json:"value_int omitempty"`
	// how do i restrict int to specific values (1,2,3) -> custom marshaler
	// value_must_be_int int `json:"value_must_be_int"`
}

type Group struct {
	union_type string  `json:"baml_type" enum:"string,int,null"`
	Result1    *Result `json:"result1"`
	Result2    *Result `json:"result2"`
}

type Item struct {
	Title string `json:"title"`
	Body  string `json:"body"`
}

type Article struct {
	Title      string    `json:"title"`
	Body       string    `json:"body"`
	SubArticle []Article `json:"sub_article"`
}

func (r *Result) IsString() bool {
	return r.baml_type == "string"
}

func (r *Result) IsInt() bool {
	return r.baml_type == "int"
}

// custom unmarshaler
func (r *Result) UnmarshalJSON(data []byte) error {
	var v map[string]interface{}
	if err := json.Unmarshal(data, &v); err != nil {
		return err
	}
	if v["baml_type"] == "string" {
		r.baml_type = "string"
		value, ok := v["value"].(string)
		if !ok {
			return fmt.Errorf("value is not a string")
		}
		r.type_string = &value
	} else if v["baml_type"] == "int" {
		r.baml_type = "int"
		value, ok := v["value"].(int)
		if !ok {
			return fmt.Errorf("value is not an int")
		}
		r.type_int = &value
	}
	return nil
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
