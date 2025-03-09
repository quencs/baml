package main

/*
#cgo LDFLAGS: ./lib/libhello.dylib -ldl
#include "./lib/baml.h"
#include <stdlib.h>
#include <stdbool.h>
*/
import "C"

import (
	"math/rand"
	"sync"
)

type ResultCallback struct {
	data string // JSON string
}

// Map to store callbacks by ID
var (
	dynamicCallbacks = make(map[uint32]chan ResultCallback)
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
		callback <- ResultCallback{data: my_string}
		if isDone {
			close(callback)
			callbackMutex.Lock()
			defer callbackMutex.Unlock()
			delete(dynamicCallbacks, id_uint)
		}
	}
}

func create_unique_id() (C.uint32_t, chan ResultCallback) {
	callbackMutex.Lock()
	defer callbackMutex.Unlock()
	id := uint32(rand.Intn(1000000))
	for _, exists := dynamicCallbacks[id]; exists; {
		id = uint32(rand.Intn(1000000))
	}
	dynamicCallbacks[id] = make(chan ResultCallback)
	return C.uint32_t(id), dynamicCallbacks[id]
}
