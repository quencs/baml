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
    "os"
    "strconv"
    "sync"
    "unsafe"
)

// Map to store callbacks by ID
var (
    dynamicCallbacks = make(map[uint32]func(*C.char))
    callbackMutex    sync.RWMutex
    resultChan       chan struct{} // Channel to signal completion
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

func invokeRuntimeCli() {
    args := os.Args
    argc := len(args)
    cArgs := make([]*C.char, argc+1)
    for i, s := range args {
        cArgs[i] = C.CString(s)
    }
    cArgs[argc] = nil
    C.invoke_runtime_cli((**C.char)(unsafe.Pointer(&cArgs[0])))
    for i := 0; i < argc; i++ {
        C.free(unsafe.Pointer(cArgs[i]))
    }
}
func main() {
    // Example: call invoke_runtime_cli with os.Args.

    // --- Now call TestOllama function ---

    // 1. Create the Baml runtime.
    runtime := C.create_baml_runtime()
    if runtime == nil {
        println("Failed to create Baml runtime")
        return
    }
    // Ensure the runtime is destroyed at the end.
    defer C.destroy_baml_runtime(runtime)

    // 2. Prepare the function name "TestOllama".
    funcName := C.CString("TestOllama")
    defer C.free(unsafe.Pointer(funcName))

    // 3. Prepare the argument for TestOllama.
    // Assume TestOllama expects one parameter "input" of type string.
    input := "Hello from Go"
    // JSON-encode the input string (for proper deserialization in Rust).
    jsonInput := strconv.Quote(input) // e.g., becomes "\"Hello from Go\""
    cValue := C.CString(jsonInput)
    defer C.free(unsafe.Pointer(cValue))

    // The key for our argument.
    key := C.CString("input")
    defer C.free(unsafe.Pointer(key))

    // 4. Build arrays for keys and values.
    keys := []*C.char{key}
    values := []*C.char{cValue}

    // 5. Create the CKwargs struct.
    var kwargs C.CKwargs
    kwargs.len = 1
    kwargs.keys = (**C.char)(unsafe.Pointer(&keys[0]))
    kwargs.values = (**C.char)(unsafe.Pointer(&values[0]))

    // 6. Prepare to wait for the callback.
    resultChan = make(chan struct{})
    
    // Use the callback registration system
    callbackID := uint32(1) // Using ID 1 for this callback
    registerCallback(callbackID, func(result *C.char) {
        res := C.GoString(result)
        println("Result from TestOllama:", res)
        // Signal completion
        resultChan <- struct{}{}
    })
    
    // Create a CString for the result callback function name
    callback := C.CString("trampolineCallback1")
    defer C.free(unsafe.Pointer(callback))

    // 7. Call the function via the Rust CFFI layer using the registered callback
    C.call_function_from_c(
        runtime, 
        funcName, 
        &kwargs,
        (C.ResultCallback)(C.callback_func(C.trampolineCallback1)),
    )

    // Wait until the callback signals completion.
    <-resultChan
    
    // Clean up by unregistering the callback
    unregisterCallback(callbackID)

    println("TestOllama function completed")
}