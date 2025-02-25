#ifndef RUST_CFFI_H
#define RUST_CFFI_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

/* 
 * Struct representing keyword arguments passed from C to Rust.
 * - `len` is the number of key/value pairs.
 * - `keys` is an array of null-terminated strings (the keys).
 * - `values` is an array of null-terminated strings (the JSON-encoded values).
 */
typedef struct CKwargs {
    size_t len;
    const char **keys;
    const char **values;
} CKwargs;

/*
 * Callback function type.
 * The callback receives a pointer to a null-terminated C string containing the JSON result.
 * Note: The returned string is allocated by Rust and must be freed using free_string.
 */
typedef void (*ResultCallback)(const char *result);

/*
 * Extern "C" functions exported from the Rust CFFI layer.
 */

// Prints a hello message. `name` must be a null-terminated string.
void hello(const char *name);

// Prints a whispered message. `message` must be a null-terminated string.
void whisper(const char *message);

// Creates and returns a pointer to a Baml runtime instance.
const void* create_baml_runtime(void);

// Destroys a previously created Baml runtime instance.
void destroy_baml_runtime(const void *runtime);

/*
 * Calls a function in the Baml runtime.
 *
 * Parameters:
 *  - runtime: a pointer to the runtime (as returned by create_baml_runtime).
 *  - function_name: the name of the function to call (null-terminated string).
 *  - kwargs: pointer to a CKwargs structure containing keyword arguments.
 *  - callback: a function to be called with the result.
 *
 * The callback receives a pointer to a C string (JSON) that must later be freed with free_string.
 */
void call_function_from_c(const void *runtime,
                          const char *function_name,
                          const CKwargs *kwargs,
                          ResultCallback callback);

// Invokes the runtime CLI. `args` is a null-terminated array of null-terminated C strings.
void invoke_runtime_cli(const char * const* args);

/*
 * Frees a C string that was allocated by the Rust runtime (e.g., in call_function_from_c).
 * Call this function on any string returned via a callback once it is no longer needed.
 */
void free_string(char *s);


// In baml.h
typedef void (*callback_func)(char*);
extern bool register_callback(uint32_t id, callback_func callback);
extern bool unregister_callback(uint32_t id);
extern bool trigger_callback(uint32_t id, char* message);

#ifdef __cplusplus
}
#endif

#endif // RUST_CFFI_H
