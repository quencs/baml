package baml

/*
#cgo LDFLAGS: ./lib/libhello.dylib -ldl
#include <stdlib.h>
#include <stdbool.h>
*/
import "C"

import (
	"context"
	"math/rand"
	"sync"
)

type ResultCallback struct {
	data string // JSON string
}

func (r *ResultCallback) Raw() string {
	return r.data
}

type CallbackData struct {
	channel chan ResultCallback
	ctx     context.Context
}

// Map to store callbacks by ID
var (
	dynamicCallbacks = make(map[uint32]CallbackData)
	callbackMutex    sync.RWMutex
)

//export trigger_callback
func trigger_callback(id C.uint32_t, isDone C.bool, result *C.char) {
	callbackMutex.RLock()
	id_uint := uint32(id)
	callback, exists := dynamicCallbacks[id_uint]
	callbackMutex.RUnlock()

	if exists {
		my_string := C.GoString(result)
		force_close := false

		select {
		case <-callback.ctx.Done():
			force_close = true
			// TODO: Somehow tell rust to die
			break
		case callback.channel <- ResultCallback{data: my_string}:
			break
		}

		if bool(isDone) || force_close {
			close(callback.channel)
			callbackMutex.Lock()
			defer callbackMutex.Unlock()
			delete(dynamicCallbacks, id_uint)
		}
	}
}

func create_unique_id(ctx context.Context) (C.uint32_t, chan ResultCallback) {
	callbackMutex.Lock()
	defer callbackMutex.Unlock()
	id := uint32(rand.Intn(1000000))
	for _, exists := dynamicCallbacks[id]; exists; {
		id = uint32(rand.Intn(1000000))
	}
	dynamicCallbacks[id] = CallbackData{channel: make(chan ResultCallback), ctx: ctx}
	return C.uint32_t(id), dynamicCallbacks[id].channel
}
