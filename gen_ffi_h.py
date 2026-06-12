import os
ffi_src = "target/jna-src/jna-5.13.0/native/libffi"
ffi_build = "target/jna-build"
os.makedirs(f"{ffi_build}/include", exist_ok=True)

# Create ffitarget.h for x86
target_h = """#ifndef FFITARGET_H
#define FFITARGET_H
#ifdef __i386__
#define FFI_SYSV 1
#define FFI_DEFAULT_ABI FFI_SYSV
#define FFI_TRAMPOLINE_SIZE 24
#endif
#endif
"""
with open(f"{ffi_build}/include/ffitarget.h", "w") as f:
    f.write(target_h)

# Create ffi.h
ffi_h = """#ifndef FFI_H
#define FFI_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif

#define FFI_OK 0
#define FFI_BAD_TYPEDEF 1
#define FFI_BAD_ABI 2

typedef struct _ffi_type ffi_type;
struct _ffi_type {
    size_t size;
    unsigned short alignment;
    unsigned short type;
    ffi_type **elements;
};

typedef enum {
    FFI_FIRST_ABI = 0,
    FFI_SYSV = 1,
    FFI_DEFAULT_ABI = FFI_SYSV
} ffi_abi;

typedef struct {
    ffi_type **arg_types;
    int nargs;
    ffi_type *rtype;
    unsigned int flags;
    unsigned int abi;
    unsigned int bytes;
    unsigned int nfixedargs;
} ffi_cif;

typedef void (*ffi_fun)(ffi_cif *, void *, void **, void *);

typedef struct {
    ffi_cif *cif;
    ffi_fun fun;
    void *user_data;
} ffi_closure;

int ffi_prep_cif(ffi_cif *cif, ffi_abi abi, unsigned int nargs, ffi_type *rtype, ffi_type **arg_types);
void ffi_call(ffi_cif *cif, void *fn, void *rvalue, void **avalue);
ffi_closure *ffi_closure_alloc(size_t size, void **code);
void ffi_closure_free(ffi_closure *closure);
int ffi_prep_closure_loc(ffi_closure *closure, ffi_cif *cif, ffi_fun fun, void *user_data, void *code);

#ifdef __cplusplus
}
#endif
#endif
"""
with open(f"{ffi_build}/include/ffi.h", "w") as f:
    f.write(ffi_h)

print("Headers created")
