// test/manifold-link/libcxx-extras.cpp
//
// Provides the libc++ source-file symbols that manifold pulls in but
// the main libcxx component intentionally doesn't ship:
//   * std::__1::__shared_count / __shared_weak_count out-of-line
//     methods (from libc++'s src/memory.cpp)
//   * std::nothrow global, std::__throw_bad_alloc (from libc++'s
//     src/new_helpers.cpp)
//   * std::bad_weak_ptr key functions
//   * std::align helper
//
// These are documented in libcxx/README.md as "consumers will surface
// link errors and need to compile [the upstream libc++ source files]
// themselves." This file IS that compilation, scoped to the manifold-
// link smoke. It's NOT part of the main libcxx component because it
// requires `#include <memory>` / `<new>` which we deliberately keep
// out of the main libcxx stubs (insulation from libc++ version
// drift). Including those headers couples this TU to libc++'s
// `__shared_count` / `__shared_weak_count` class layout — if libc++
// changes those layouts in a future release, this TU silently breaks
// (the manifold-tests run will catch it). We accept that coupling
// because the alternative — redeclaring the classes locally with
// matching layouts — is fragile in a different way.
//
// Runtime semantics: refcount manipulation is non-atomic (correct
// for our single-threaded wasm), and the destructor + __release_weak
// dispatch through the vtable to derived-class
// `__on_zero_shared_weak()` implementations — which DO free the
// controlled object. Lifetime correctness holds for single-threaded
// use; do not assume it generalizes to a future threaded build.
//
// References (the upstream files we're emulating):
//   https://github.com/llvm/llvm-project/blob/release/20.x/libcxx/src/memory.cpp
//   https://github.com/llvm/llvm-project/blob/release/20.x/libcxx/src/new_helpers.cpp

#include <memory>
#include <new>
#include <typeinfo>

// ---- std::nothrow + __throw_bad_alloc (from new_helpers.cpp) ----

namespace std {  // intentionally NOT versioned (matches libc++ upstream)

const nothrow_t nothrow{};

[[noreturn]] void __throw_bad_alloc() {
    __builtin_trap();
}

}  // namespace std

// ---- bad_weak_ptr key functions + shared_count/__shared_weak_count
//      out-of-line methods (from memory.cpp) ----

namespace std { inline namespace __1 {

bad_weak_ptr::~bad_weak_ptr() noexcept {}
const char* bad_weak_ptr::what() const noexcept { return "bad_weak_ptr"; }

__shared_count::~__shared_count() {}
__shared_weak_count::~__shared_weak_count() {}

void __shared_weak_count::__release_weak() noexcept {
    if (--__shared_weak_owners_ == -1) {
        __on_zero_shared_weak();
    }
}

__shared_weak_count* __shared_weak_count::lock() noexcept {
    long object_owners = __shared_owners_;
    while (object_owners != -1) {
        // single-threaded: just bump and return
        __shared_owners_ = object_owners + 1;
        return this;
    }
    return nullptr;
}

const void* __shared_weak_count::__get_deleter(const type_info&) const noexcept {
    return nullptr;
}

// std::align (rare-use helper, vendored from upstream verbatim)
void* align(size_t alignment, size_t size, void*& ptr, size_t& space) {
    void* r = nullptr;
    if (size <= space) {
        char* p1 = static_cast<char*>(ptr);
        char* p2 = reinterpret_cast<char*>(
            reinterpret_cast<__UINTPTR_TYPE__>(p1 + (alignment - 1)) & -alignment);
        size_t d = static_cast<size_t>(p2 - p1);
        if (d <= space - size) {
            r = p2;
            ptr = r;
            space -= d;
        }
    }
    return r;
}

}}  // namespace std::__1
