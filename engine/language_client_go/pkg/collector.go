package baml

import (
	"runtime"
	"unsafe"

	"github.com/boundaryml/baml/engine/language_client_go/baml_go"
)

type Collector struct {
	c unsafe.Pointer
}

type Usage struct {
	c unsafe.Pointer
}

const collectorType = "collector"
const usageType = "usage"

func NewCollector() *Collector {
	collectorPtr, err := baml_go.CallCollectorFunction(nil, collectorType, "new")
	if err != nil {
		panic(err)
	}

	collector := &Collector{
		c: collectorPtr,
	}

	runtime.AddCleanup(collector, func(c unsafe.Pointer) {
		baml_go.CallCollectorFunction(c, collectorType, "destroy")
	}, collector.c)

	return collector
}

func (c *Collector) Usage() (*Usage, error) {
	usagePtr, err := baml_go.CallCollectorFunction(c.c, collectorType, "usage")
	if err != nil {
		return nil, err
	}

	usage := &Usage{
		c: usagePtr,
	}

	runtime.AddCleanup(usage, func(c unsafe.Pointer) {
		baml_go.CallCollectorFunction(c, collectorType, "destroy")
	}, usage.c)

	return usage, nil
}

func (u *Usage) InputTokens() (int, error) {
	inputTokensPtr, err := baml_go.CallCollectorFunction(u.c, usageType, "input_tokens")
	if err != nil {
		return 0, err
	}

	return int(uintptr(unsafe.Pointer(inputTokensPtr))), nil
}

func (u *Usage) OutputTokens() (int, error) {
	outputTokensPtr, err := baml_go.CallCollectorFunction(u.c, usageType, "output_tokens")
	if err != nil {
		return 0, err
	}

	return int(uintptr(unsafe.Pointer(outputTokensPtr))), nil
}
