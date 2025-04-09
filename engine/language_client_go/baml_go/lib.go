package baml_go

/*
#cgo CFLAGS: -I${SRCDIR}/../include
#cgo CFLAGS: -O3 -g
#cgo LDFLAGS: -ldl
#include <dlfcn.h>
#include <baml_cffi_wrapper.h>
#include <stdlib.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
*/
import "C"

import (
	"errors"
	"fmt"
	"os"
	"runtime"
	"unsafe"
)

// BAML specific error messages
var (
	ErrLoadLibrary          = errors.New("failed loading BAML shared library")
	ErrNotSupportedPlatform = errors.New("supported only Linux and MacOS")
)

var bamlSharedLibraryPath = ""

// SetSharedLibraryPath sets the library path for BAML.
func SetSharedLibraryPath(path string) {
	bamlSharedLibraryPath = path
}

// initialization initializes the BAML shared library.
func initialization() error {
	if !checkPlatform() {
		return ErrNotSupportedPlatform
	}

	if err := initSharedLibraryPath(); err != nil {
		return err
	}

	cName := C.CString(bamlSharedLibraryPath)
	defer C.free(unsafe.Pointer(cName))

	handle := C.dlopen(cName, C.RTLD_LAZY)
	if handle == nil {
		msg := C.GoString(C.dlerror())
		return fmt.Errorf("%w `%s`: %s", ErrLoadLibrary, bamlSharedLibraryPath, msg)
	}

	lib := library{handle}

	// Register functions from the BAML shared library
	lib.registerFn("Version")
	lib.registerFn("CreateBamlRuntime")
	lib.registerFn("DestroyBamlRuntime")
	lib.registerFn("InvokeRuntimeCli")
	lib.registerFn("RegisterCallbacks")
	lib.registerFn("CallFunctionFromC")
	lib.registerFn("CallFunctionStreamFromC")
	lib.registerFn("Callback")

	return nil
}

// library represents a loaded shared library.
type library struct {
	handle unsafe.Pointer
}

// registerFn registers a function from the shared library.
func (l *library) registerFn(fnName string) {
	fnC := getFromLibraryFn(l.handle, fnName)

	switch fnName {
	case "Version":
		C.SetVersionFn(fnC)
	case "CreateBamlRuntime":
		C.SetCreateBamlRuntimeFn(fnC)
	case "DestroyBamlRuntime":
		C.SetDestroyBamlRuntimeFn(fnC)
	case "InvokeRuntimeCli":
		C.SetInvokeRuntimeCliFn(fnC)
	case "RegisterCallbacks":
		C.SetRegisterCallbacksFn(fnC)
	case "CallFunctionFromC":
		C.SetCallFunctionFromCFn(fnC)
	case "CallFunctionStreamFromC":
		C.SetCallFunctionStreamFromCFn(fnC)
	default:
		panic(fmt.Sprintf("not supported function from BAML library: %s", fnName))
	}
}

// initSharedLibraryPath initializes the shared library path.
func initSharedLibraryPath() error {
	if bamlSharedLibraryPath == "" {
		bamlSharedLibraryPath = os.Getenv("BAML_LIBRARY_PATH")
	}

	if bamlSharedLibraryPath == "" {
		switch runtime.GOOS {
		case "darwin":
			bamlSharedLibraryPath = "/usr/local/lib/libbaml.dylib"
		case "linux":
			bamlSharedLibraryPath = "/usr/local/lib/libbaml.so"
		default:
			panic("Unsupported OS for BAML shared library")
		}
	}

	if _, err := os.Stat(bamlSharedLibraryPath); errors.Is(err, os.ErrNotExist) {
		return fmt.Errorf("%w: %s", ErrLoadLibrary, bamlSharedLibraryPath)
	}

	return nil
}

// checkPlatform checks if the current platform is supported.
func checkPlatform() bool {
	return runtime.GOOS == "darwin" || runtime.GOOS == "linux"
}

// getFromLibraryFn retrieves a function pointer from the shared library.
func getFromLibraryFn(handle unsafe.Pointer, fnName string) unsafe.Pointer {
	cFnName := C.CString(fnName)
	defer C.free(unsafe.Pointer(cFnName))

	fn := C.dlsym(handle, cFnName)
	if fn == nil {
		msg := C.GoString(C.dlerror())
		panic(fmt.Sprintf("Error looking up %s in `%s`: %s", fnName, bamlSharedLibraryPath, msg))
	}

	return fn
}
