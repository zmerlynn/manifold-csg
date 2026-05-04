//! Cooperative cancellation + progress observation for long-running boolean evaluations.
//!
//! Manifold operations are lazy: building a CSG tree is cheap, and the
//! actual evaluation happens when you query results (e.g., via
//! [`Manifold::status_with_context`](crate::Manifold::status_with_context),
//! `num_tri`, mesh extraction, etc.). An [`ExecutionContext`] lets you
//! observe an in-flight evaluation from another thread and ask it to stop
//! early.
//!
//! Cancellation is **sticky** (once cancelled, stays cancelled) and granular
//! per-boolean (the upstream kernel checks the cancel flag at boolean
//! boundaries; it doesn't interrupt a single boolean mid-flight). Progress
//! is reported as a fraction in `[0.0, 1.0]`.
//!
//! The C API documents the underlying `ExecutionContext` as safe to read
//! and write from any thread, so [`ExecutionContext`] is `Send` + `Sync`
//! and can be wrapped in [`Arc`](std::sync::Arc) to share between the
//! evaluator thread and a controller/observer thread.
//!
//! ```no_run
//! use std::sync::Arc;
//! use std::thread;
//! use std::time::Duration;
//! use manifold_csg::{ExecutionContext, Manifold};
//!
//! let ctx = Arc::new(ExecutionContext::new());
//! let cancel = Arc::clone(&ctx);
//!
//! // Cancel the evaluation if it takes longer than 100ms.
//! thread::spawn(move || {
//!     thread::sleep(Duration::from_millis(100));
//!     cancel.cancel();
//! });
//!
//! let result = Manifold::cube(1.0, 1.0, 1.0, true);
//! let status = result.status_with_context(&ctx);
//! // `status` will be `NoError` for trivial work that finishes before
//! // cancel fires; for a heavy boolean tree it would surface cancellation.
//! # let _ = status;
//! ```
//!
//! Available since manifold3d's post-v3.4.1 master.

use manifold_csg_sys::{
    ManifoldExecutionContext, manifold_alloc_execution_context, manifold_delete_execution_context,
    manifold_execution_context, manifold_execution_context_cancel,
    manifold_execution_context_cancelled, manifold_execution_context_progress,
};

/// Observes progress and allows cooperative cancellation of long-running
/// boolean evaluations. See the [module docs](self) for usage.
pub struct ExecutionContext {
    ptr: *mut ManifoldExecutionContext,
}

// SAFETY: The C API explicitly documents `ExecutionContext` as safe to
// read/write from any thread; the upstream C++ implementation
// synchronizes the cancel flag and progress counter internally.
unsafe impl Send for ExecutionContext {}
// SAFETY: Same justification as `Send` — all accessors (`cancel`,
// `cancelled`, `progress`) are documented thread-safe at the C boundary.
unsafe impl Sync for ExecutionContext {}

impl ExecutionContext {
    /// Create a fresh, un-cancelled context with zero progress.
    #[must_use]
    pub fn new() -> Self {
        // SAFETY: alloc returns a valid handle; the constructor takes that
        // raw memory and constructs an ExecutionContext into it (returning
        // the same pointer).
        let mem = unsafe { manifold_alloc_execution_context() };
        // SAFETY: mem is a valid, freshly-allocated handle.
        let ptr = unsafe { manifold_execution_context(mem) };
        Self { ptr }
    }

    /// Request cancellation of any in-flight evaluation observing this
    /// context. Sticky: once called, [`is_cancelled`](Self::is_cancelled)
    /// returns `true` for the rest of the context's lifetime.
    pub fn cancel(&self) {
        // SAFETY: self.ptr is a valid handle for the lifetime of self;
        // upstream documents thread-safe access.
        unsafe { manifold_execution_context_cancel(self.ptr) };
    }

    /// Returns `true` if [`cancel`](Self::cancel) has been called.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        // SAFETY: self.ptr is a valid handle; upstream documents thread-safe access.
        unsafe { manifold_execution_context_cancelled(self.ptr) != 0 }
    }

    /// Progress of an in-flight evaluation as a fraction in `[0.0, 1.0]`.
    /// Reads `0.0` before any evaluation has started and `1.0` after one
    /// has finished.
    #[must_use]
    pub fn progress(&self) -> f64 {
        // SAFETY: self.ptr is a valid handle; upstream documents thread-safe access.
        unsafe { manifold_execution_context_progress(self.ptr) }
    }

    /// Raw pointer for FFI calls that take an `ExecutionContext`. Crate-local
    /// so the safe wrapper retains exclusive control of the lifetime.
    pub(crate) fn as_ptr(&self) -> *mut ManifoldExecutionContext {
        self.ptr
    }
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ExecutionContext {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // SAFETY: ptr was returned by alloc + construct; not freed since.
            unsafe { manifold_delete_execution_context(self.ptr) };
            self.ptr = std::ptr::null_mut();
        }
    }
}
