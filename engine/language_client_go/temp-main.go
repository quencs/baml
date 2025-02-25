package main

/*
#cgo LDFLAGS: ./lib/libhello.dylib -ldl
#include "./lib/baml.h"
#include <stdlib.h>

// Declare the callback type
typedef void (*callback_func)(char*);

// Export callbacks
extern void trampolineCallback1(char* result);
extern void trampolineCallback2(char* result);
extern void trampolineCallback3(char* result);
*/
import "C"
import (
    "sync"
    "unsafe"
)

// Map to store callbacks by ID
var (
    dynamicCallbacks = make(map[uint32]func(*C.char))
    callbackMutex    sync.RWMutex
)

//export trampolineCallback1
func trampolineCallback1(result *C.char) {
    handleCallback(1, result)
}

//export trampolineCallback2
func trampolineCallback2(result *C.char) {
    handleCallback(2, result)
}

//export trampolineCallback3
func trampolineCallback3(result *C.char) {
    handleCallback(3, result)
}

func handleCallback(id uint32, result *C.char) {
    callbackMutex.RLock()
    callback, exists := dynamicCallbacks[id]
    callbackMutex.RUnlock()
    
    if exists {
        callback(result)
    }
}

func registerCallback(id uint32, callback func(*C.char)) bool {
    callbackMutex.Lock()
    dynamicCallbacks[id] = callback
    callbackMutex.Unlock()
    
    // Register the appropriate trampoline based on ID
    var success C.bool
    switch id {
    case 1:
        success = C.register_callback(C.uint(id), (C.callback_func)(C.trampolineCallback1))
    case 2:
        success = C.register_callback(C.uint(id), (C.callback_func)(C.trampolineCallback2))
    case 3:
        success = C.register_callback(C.uint(id), (C.callback_func)(C.trampolineCallback3))
    default:
        return false
    }
    
    return bool(success)
}

func unregisterCallback(id uint32) bool {
    callbackMutex.Lock()
    delete(dynamicCallbacks, id)
    callbackMutex.Unlock()
    
    return bool(C.unregister_callback(C.uint(id)))
}

func main() {
    // Register multiple callbacks
    registerCallback(1, func(result *C.char) {
        res := C.GoString(result)
        println("Callback 1 received:", res)
    })
    
    registerCallback(2, func(result *C.char) {
        res := C.GoString(result)
        println("Callback 2 received:", res)
    })
    
    // Test triggering different callbacks
    message1 := C.CString("Test message for callback 1")
    defer C.free(unsafe.Pointer(message1))
    C.trigger_callback(C.uint(1), message1)
    
    message2 := C.CString("Test message for callback 2")
    defer C.free(unsafe.Pointer(message2))
    C.trigger_callback(C.uint(2), message2)
    
    // Unregister a callback when done
    unregisterCallback(1)
    
    println("All callbacks processed")
}