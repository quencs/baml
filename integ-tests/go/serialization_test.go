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
		{`{ "class_name": "Blah", "values": { "Prop4": "test" } }`, &types.Blah{Prop4: &[]string{"test"}[0]}},
		{`{ "class_name": "InputClassNested", "values": { "Key": "test", "Nested": { "class_name": "InputClass", "values": { "Key": "test" } } } }`, &types.InputClassNested{Key: "test", Nested: types.InputClass{Key: "test"}}},
		{`{ "class_name": "Education", "values": { "Institution": "test", "Location": "test", "Degree": "test", "Major": ["test"], "Graduation_date": "test" } }`, &types.Education{Institution: "test", Location: "test", Degree: "test", Major: []string{"test"}, Graduation_date: &[]string{"test"}[0]}},
		{`{ "class_name": "Forest", "values": { "Trees": [{"class_name": "Tree", "values": { "Children": { "class_name": "Forest", "values": { "Trees": [] } } }}] } }`, &types.Forest{Trees: []types.Tree{{Children: types.Forest{Trees: []types.Tree{}}}}}},
		{`[{ "key": "testKey", "value": "testValue" }]`, map[string]any{"testKey": "testValue"}},
		{`{ "class_name": "BookOrder", "values": { "OrderId": "123", "Title": "test", "Quantity": 1, "Price": 12.3 } }`, &types.BookOrder{OrderId: "123", Title: "test", Quantity: 1, Price: 12.3}},
		{`{ "enum_class": "Category", "enum_value": "Refund" }`, types.CategoryRefund},
		{`{ "class_name": "TestClassWithEnum", "values": { "Prop1": "test", "Prop2": { "enum_class": "EnumInClass", "enum_value": "ONE" } } }`, &types.TestClassWithEnum{Prop1: "test", Prop2: types.EnumInClassONE}},
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
