package main

import (
	"encoding/json"
	_ "fmt"
	"testing"

	"github.com/stretchr/testify/require"

	"example.com/integ-tests/baml_client"
	"example.com/integ-tests/baml_client/types"
)

var testCases = []struct {
	input    string
	expected any
}{
	{`"test"`, "test"},
	{`12`, float64(12)},
	{`12.3`, float64(12.3)},
	{`true`, true},
	{`false`, false},
	{`{ "class_name": "Blah", "values": { "Prop4": "test" } }`, &types.Blah{Prop4: &[]string{"test"}[0]}},
	{`{ "class_name": "InputClassNested", "values": { "Key": "test", "Nested": { "class_name": "InputClass", "values": { "Key": "test", "Key2": "" } } } }`, &types.InputClassNested{Key: "test", Nested: types.InputClass{Key: "test", Key2: ""}}},
	{`{ "class_name": "Education", "values": { "Institution": "test", "Location": "test", "Degree": "test", "Major": ["test"], "Graduation_date": "test" } }`, &types.Education{Institution: "test", Location: "test", Degree: "test", Major: []string{"test"}, Graduation_date: &[]string{"test"}[0]}},
	{`{ "class_name": "Forest", "values": { "Trees": [{"class_name": "Tree", "values": {  "Data": 0, "Children": { "class_name": "Forest", "values": { "Trees": [] } } }}] } }`, &types.Forest{Trees: []types.Tree{{Children: types.Forest{Trees: []types.Tree{}}}}}},
	{`[{ "key": "testKey", "value": "testValue" }]`, map[string]any{"testKey": "testValue"}},
	{`{ "class_name": "BookOrder", "values": { "OrderId": "123", "Title": "test", "Quantity": 1, "Price": 12.3 } }`, &types.BookOrder{OrderId: "123", Title: "test", Quantity: 1, Price: 12.3}},
	{`{ "enum_class": "Category", "enum_value": "Refund" }`, types.CategoryRefund},
	{`{ "class_name": "TestClassWithEnum", "values": { "Prop1": "test", "Prop2": { "enum_class": "EnumInClass", "enum_value": "ONE" } } }`, &types.TestClassWithEnum{Prop1: "test", Prop2: types.EnumInClassONE}},
	{`{ "union_name": "Union__string__int64__float64", "union_variant": "string", "value": "test" }`, types.Union__string__int64__float64NewWithString(&[]string{"test"}[0])},
}

// This test should probably be 'generated' optionally instead of here, as there is no reason to
// expose `Decode` in the public API.
func TestDeserialization(t *testing.T) {
	for _, test := range testCases {
		t.Run(test.input, func(t *testing.T) {
			actual, err := baml_client.Decode([]byte(test.input))
			require.NoError(t, err)
			require.Equal(t, test.expected, actual)
		})
	}
}

func TestSerialization(t *testing.T) {
	for _, test := range testCases {

		// ignore this enum test for now because it's not supported yet
		if test.expected == types.CategoryRefund {
			continue
		}

		t.Run(test.input, func(t *testing.T) {
			actual, err := baml_client.Encode(test.expected)
			require.NoError(t, err)

			compareJSON(t, []byte(test.input), actual)
		})
	}
}

func TestRoundTripFromInput(t *testing.T) {
	for _, test := range testCases {
		if test.expected == types.CategoryRefund {
			continue
		}

		t.Run(test.input, func(t *testing.T) {
			decoded, err := baml_client.Decode([]byte(test.input))
			require.NoError(t, err)

			roundTrippedData, err := baml_client.Encode(decoded)
			require.NoError(t, err)

			compareJSON(t, []byte(test.input), roundTrippedData)
		})
	}
}

func TestRoundTripFromOutput(t *testing.T) {
	for _, test := range testCases {
		t.Run(test.input, func(t *testing.T) {
			encoded, err := baml_client.Encode(test.expected)
			require.NoError(t, err)

			decoded, err := baml_client.Decode(encoded)
			require.NoError(t, err)

			require.Equal(t, test.expected, decoded)
		})
	}
}

// can't directly compare the JSON strings because the order of keys and whitespace is not guaranteed
func compareJSON(t *testing.T, expected, actual []byte) {
	t.Helper()

	var expectedObject any
	err := json.Unmarshal(expected, &expectedObject)
	require.NoError(t, err)

	var actualObject any
	err = json.Unmarshal(actual, &actualObject)
	require.NoError(t, err)

	require.Equal(t, expectedObject, actualObject)
}
