package main

import (
	_ "fmt"
	"testing"

	"github.com/stretchr/testify/require"

	"example.com/integ-tests/baml_client"
	"example.com/integ-tests/baml_client/types"
)

// This test should probably be 'generated' optionally instead of here, as there is no reason to
// expose `Decode` in the public API.
func TestDeserialization(t *testing.T) {
	var deserializationTests = []struct {
		input    string
		expected any
	}{
		{`"test"`, "test"},
		{`12`, float64(12)},
		{`12.3`, float64(12.3)},
		{`true`, true},
		{`false`, false},
		{`{ "class_name": "Blah", "Prop4": "test" }`, &types.Blah{Prop4: &[]string{"test"}[0]}},
	}

	for _, test := range deserializationTests {
		t.Run(test.input, func(t *testing.T) {
			actual, err := baml_client.Decode([]byte(test.input))
			if err != nil {
				t.Errorf("expected no error, got %v", err)
			}
			require.Equal(t, test.expected, actual)
		})
	}
}
