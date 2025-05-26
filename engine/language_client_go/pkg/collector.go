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

func (c *Collector) pointer() unsafe.Pointer {
	return c.c
}

func (c *Collector) rustType() string {
	return collectorType
}

type Usage struct {
	c unsafe.Pointer
}

func (u *Usage) pointer() unsafe.Pointer {
	return u.c
}

func (u *Usage) rustType() string {
	return usageType
}

func createUsage(c unsafe.Pointer) *Usage {
	return &Usage{
		c: c,
	}
}

type FunctionLog struct {
	c unsafe.Pointer
}

func (f *FunctionLog) pointer() unsafe.Pointer {
	return f.c
}

func (f *FunctionLog) rustType() string {
	return functionLogType
}

func createFunctionLog(c unsafe.Pointer) *FunctionLog {
	return &FunctionLog{
		c: c,
	}
}

type Timing struct {
	c unsafe.Pointer
}

func (t *Timing) pointer() unsafe.Pointer {
	return t.c
}

func (t *Timing) rustType() string {
	return timingType
}

func createTiming(c unsafe.Pointer) *Timing {
	return &Timing{
		c: c,
	}
}

type LLMCall struct {
	c unsafe.Pointer
}

func (l *LLMCall) pointer() unsafe.Pointer {
	return l.c
}

func (l *LLMCall) rustType() string {
	return llmCallType
}

func createLLMCall(c unsafe.Pointer) *LLMCall {
	return &LLMCall{
		c: c,
	}
}

type rustPointer interface {
	pointer() unsafe.Pointer
	rustType() string
}

func wrap[T rustPointer](createFn func(c unsafe.Pointer) T, c unsafe.Pointer) T {
	wrapped := createFn(c)

	runtime.AddCleanup(&wrapped, func(ptr unsafe.Pointer) {
		baml_go.CallCollectorFunction(ptr, wrapped.rustType(), "destroy")
	}, wrapped.pointer())

	return wrapped
}

func createCollector(c unsafe.Pointer) *Collector {
	return &Collector{
		c: c,
	}
}

func convertString(ptr unsafe.Pointer) string {
	str := C.GoString((*C.char)(ptr))

	baml_go.CallCollectorFunction(ptr, stringType, "destroy")

	return str
}

func NewCollector() *Collector {
	collectorPtr, err := baml_go.CallCollectorFunction(nil, collectorType, "new")
	if err != nil {
		panic(err)
	}

	return wrap(createCollector, collectorPtr)
}

func (c *Collector) Usage() (*Usage, error) {
	usagePtr, err := baml_go.CallCollectorFunction(c.c, collectorType, "usage")
	if err != nil {
		return nil, err
	}

	return wrap(createUsage, usagePtr), nil
}

func (c *Collector) Last() (*FunctionLog, error) {
	functionLogPtr, err := baml_go.CallCollectorFunction(c.c, collectorType, "last")
	if err != nil {
		return nil, err
	}

	return wrap(createFunctionLog, functionLogPtr), nil
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

	return wrap(createUsage, usagePtr), nil
}

func (f *FunctionLog) Id() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "id")
	if err != nil {
		return "", err
	}

	return convertString(stringPtr), nil
}

func (f *FunctionLog) FunctionName() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "function_name")
	if err != nil {
		return "", err
	}

	return convertString(stringPtr), nil
}

func (f *FunctionLog) LogType() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "log_type")
	if err != nil {
		return "", err
	}

	return convertString(stringPtr), nil
}

func (f *FunctionLog) Timing() (*Timing, error) {
	timingPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "timing")
	if err != nil {
		return nil, err
	}

	return wrap(createTiming, timingPtr), nil
}

func (f *FunctionLog) RawLlmResponse() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(f.c, functionLogType, "raw_llm_response")
	if err != nil {
		return "", err
	}

	return convertString(stringPtr), nil
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

		calls = append(calls, wrap(createLLMCall, unsafe.Pointer(ptr)))
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

	return convertString(stringPtr), nil
}

func (l *LLMCall) Provider() (string, error) {
	stringPtr, err := baml_go.CallCollectorFunction(l.c, llmCallType, "provider")
	if err != nil {
		return "", err
	}

	return convertString(stringPtr), nil
}

func (l *LLMCall) Timing() (*Timing, error) {
	timingPtr, err := baml_go.CallCollectorFunction(l.c, llmCallType, "timing")
	if err != nil {
		return nil, err
	}

	return wrap(createTiming, timingPtr), nil
}

func (l *LLMCall) Usage() (*Usage, error) {
	usagePtr, err := baml_go.CallCollectorFunction(l.c, llmCallType, "usage")
	if err != nil {
		return nil, err
	}

	return wrap(createUsage, usagePtr), nil
}
