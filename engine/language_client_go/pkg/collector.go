package baml

import (
	"C"
	"runtime"
	"unsafe"

	"github.com/boundaryml/baml/engine/language_client_go/baml_go"
)

const collectorType = "collector"
const usageType = "usage"
const functionLogType = "function_log"
const stringType = "string"
const timingType = "timing"
const llmCallType = "llm_call"

type Collector struct {
	c unsafe.Pointer
}
type Usage struct {
	c unsafe.Pointer
}

type FunctionLog struct {
	c unsafe.Pointer
}

type Timing struct {
	c unsafe.Pointer
}

type LLMCall struct {
	c unsafe.Pointer
}

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
		baml_go.CallCollectorFunction(c, usageType, "destroy")
	}, usage.c)

	return usage, nil
}

func (c *Collector) Last() (*FunctionLog, error) {
	functionLogPtr, err := baml_go.CallCollectorFunction(c.c, collectorType, "last")
	if err != nil {
		return nil, err
	}

	functionLog := &FunctionLog{
		c: functionLogPtr,
	}

	runtime.AddCleanup(functionLog, func(c unsafe.Pointer) {
		baml_go.CallCollectorFunction(c, functionLogType, "destroy")
	}, functionLog.c)

	return functionLog, nil
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

func (f *FunctionLog) Usage() (*Usage, error) {
	usagePtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "usage")
	if err != nil {
		return nil, err
	}

	usage := &Usage{
		c: usagePtr,
	}

	runtime.AddCleanup(usage, func(c unsafe.Pointer) {
		baml_go.CallCollectorFunction(c, usageType, "destroy")
	}, usage.c)

	return usage, nil
}

func (f *FunctionLog) Id() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "id")
	if err != nil {
		return "", err
	}

	id := C.GoString((*C.char)(stringPtr))

	baml_go.CallCollectorFunction(stringPtr, stringType, "destroy")

	return id, nil
}

func (f *FunctionLog) FunctionName() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "function_name")
	if err != nil {
		return "", err
	}

	functionName := C.GoString((*C.char)(stringPtr))

	baml_go.CallCollectorFunction(stringPtr, stringType, "destroy")

	return functionName, nil
}

func (f *FunctionLog) LogType() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "log_type")
	if err != nil {
		return "", err
	}

	logType := C.GoString((*C.char)(stringPtr))

	baml_go.CallCollectorFunction(stringPtr, stringType, "destroy")

	return logType, nil
}

func (f *FunctionLog) Timing() (*Timing, error) {
	timingPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "timing")
	if err != nil {
		return nil, err
	}

	timing := &Timing{
		c: timingPtr,
	}

	runtime.AddCleanup(timing, func(c unsafe.Pointer) {
		baml_go.CallCollectorFunction(c, timingType, "destroy")
	}, timing.c)

	return timing, nil
}

func (f *FunctionLog) RawLlmResponse() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "raw_llm_response")
	if err != nil {
		return "", err
	}

	rawLlmResponse := C.GoString((*C.char)(stringPtr))

	baml_go.CallCollectorFunction(stringPtr, stringType, "destroy")

	return rawLlmResponse, nil
}

func (f *FunctionLog) Calls() ([]*LLMCall, error) {
	callsRaw, err := baml_go.CallCollectorFunction(f.c, functionLogType, "calls")
	if err != nil {
		return nil, err
	}

	const max = 1024

	callArray := (*[max]*C.void)(callsRaw)

	calls := make([]*LLMCall, 0)
	for i := 0; i < max; i++ {
		ptr := callArray[i]
		if ptr == nil {
			break
		}

		call := &LLMCall{c: unsafe.Pointer(ptr)}

		runtime.AddCleanup(call, func(c unsafe.Pointer) {
			baml_go.CallCollectorFunction(c, llmCallType, "destroy")
		}, call.c)

		calls = append(calls, call)
	}

	baml_go.CallCollectorFunction(callsRaw, "list", "destroy")

	return calls, nil
}

func (t *Timing) StartTimeUtcMs() (int, error) {
	startTimeUtcMsPtr, err := baml_go.CallCollectorFunction(t.c, timingType, "start_time_utc_ms")
	if err != nil {
		return 0, err
	}

	return int(uintptr(unsafe.Pointer(startTimeUtcMsPtr))), nil
}

func (t *Timing) DurationMs() (int, error) {
	durationMsPtr, err := baml_go.CallCollectorFunction(t.c, timingType, "duration_ms")
	if err != nil {
		return 0, err
	}

	return int(uintptr(unsafe.Pointer(durationMsPtr))), nil
}

func (l *LLMCall) ClientName() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(l.c, llmCallType, "client_name")
	if err != nil {
		return "", err
	}

	clientName := C.GoString((*C.char)(stringPtr))

	baml_go.CallCollectorFunction(stringPtr, stringType, "destroy")

	return clientName, nil
}

func (l *LLMCall) Provider() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(l.c, llmCallType, "provider")
	if err != nil {
		return "", err
	}

	provider := C.GoString((*C.char)(stringPtr))

	baml_go.CallCollectorFunction(stringPtr, stringType, "destroy")

	return provider, nil
}

func (l *LLMCall) Timing() (*Timing, error) {
	timingPtr, err := baml_go.CallCollectorFunction(l.c, llmCallType, "timing")
	if err != nil {
		return nil, err
	}

	timing := &Timing{
		c: timingPtr,
	}

	runtime.AddCleanup(timing, func(c unsafe.Pointer) {
		baml_go.CallCollectorFunction(c, timingType, "destroy")
	}, timing.c)

	return timing, nil
}

func (l *LLMCall) Usage() (*Usage, error) {
	usagePtr, err := baml_go.CallCollectorFunction(l.c, llmCallType, "usage")
	if err != nil {
		return nil, err
	}

	usage := &Usage{
		c: usagePtr,
	}

	runtime.AddCleanup(usage, func(c unsafe.Pointer) {
		baml_go.CallCollectorFunction(c, usageType, "destroy")
	}, usage.c)

	return usage, nil
}
