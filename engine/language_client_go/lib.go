package main

/*
#cgo LDFLAGS: ./lib/libhello.dylib -ldl
#include <stdlib.h>
#include <stdbool.h>
#include "./lib/baml.h"

extern void trigger_callback(uint32_t, bool, const char *);
*/
import "C"

import (
	"context"
	"fmt"
	"sync"
	"unsafe"
)

type StreamResult[Partial any, Final any] struct {
	partial *Partial
	final   *Final
	error   error
}

func (result *StreamResult[Partial, Final]) Partial() Partial {
	return *result.partial
}

func (result *StreamResult[Partial, Final]) Final() Final {
	return *result.final
}

func (result *StreamResult[Partial, Final]) IsFinal() bool {
	return result.final != nil
}

func (result *StreamResult[Partial, Final]) IsPartial() bool {
	return result.partial != nil
}

func (result *StreamResult[Partial, Final]) Error() error {
	return result.error
}

type DoSomethingStreamResult struct {
	StreamResult[string, string]
}

// UpperCase in Go only!
func DoSomethingStream(ctx context.Context, arg string) <-chan DoSomethingStreamResult {
	// Do something with BAML runtime here!
	// C.trigger_callback(C.uint(id), C.bool(false), C.CString(arg))

	function_name := C.CString("TestOllama")

	kwargs := C.CKwargs{
		len:    0,
		keys:   nil,
		values: nil,
	}

	callback_id, callback := create_unique_id(ctx)

	C.call_function_stream_from_c(runtime, function_name, &kwargs, callback_id)

	return_channel := make(chan DoSomethingStreamResult)
	go func() {
		for {
			select {
			case <-ctx.Done():
				close(return_channel)
				return
			case result := <-callback:
				// TODO: Handle the result
				// error handling, type checking, etc.
				return_channel <- DoSomethingStreamResult{StreamResult: StreamResult[string, string]{partial: &result.data}}
			}
		}
	}()
	return return_channel
}

var runtime unsafe.Pointer

func onClose() {
	if runtime != nil {
		C.destroy_baml_runtime(runtime)
	}
}

func init() {
	C.register_callback((C.callback_fcn)(C.trigger_callback))
	runtime = C.create_baml_runtime()
	if runtime == nil {
		panic("Failed to create Baml runtime")
	}
}

func processStream(id int, channel <-chan DoSomethingStreamResult, wg *sync.WaitGroup) {
	defer wg.Done()
	for result := range channel {
		if result.Error() != nil {
			fmt.Printf("Stream %d Error: %v\n", id, result.Error())
		} else if result.IsPartial() {
			fmt.Printf("Stream %d Partial: %s\n", id, result.Partial())
		} else if result.IsFinal() {
			fmt.Printf("Stream %d Final: %s\n", id, result.Final())
		}
	}
}

func init() {
	numStreams := 5 // Change this to run X parallel streams
	var wg sync.WaitGroup
	ctx := context.Background()

	for i := 0; i < numStreams; i++ {
		wg.Add(1)
		go processStream(i, DoSomethingStream(ctx, "Hello, world!"), &wg)
	}

	wg.Wait()
}
