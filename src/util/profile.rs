// This module's implementation has been inspired by hprof:
// <https://cmr.github.io/hprof/src/hprof/lib.rs.html>

use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use floating_duration::TimeAsFloat;

thread_local!(
    /// Global thread-local instance of the profiler.
    pub static PROFILER: RefCell<Profiler> = RefCell::new(Profiler::new())
);

/// This function must be called once at the start of each frame. The
/// resulting `Guard` must be kept alive until the end of the frame.
pub fn start_frame() -> Guard {
    PROFILER.with(|p| p.borrow_mut().start_frame())
}

/// Print profiling scope tree.
pub fn print<W: std::io::Write>(out: &mut W) {
    PROFILER.with(|p| p.borrow().print(out));
}

/// Reset profiling information.
pub fn reset() {
    PROFILER.with(|p| p.borrow_mut().reset());
}

/// Use this macro to add the current scope to profiling. In effect, the time
/// taken from entering to leaving the scope will be measured.
///
/// Internally, the scope is added as a `Scope` to the global `PROFILER`.
///
/// # Example
/// The following example will profile the scope "foo", which has the scope
/// "bar" as a child.
///
/// ```
/// {
///     profile!("foo");
///
///     {
///         profile!("bar");
///         // ... do something ...
///     }
///
///     // ... do some more ...
/// }
/// ```
#[macro_export]
macro_rules! profile {
    ($name:expr) => {
        let _guard = $crate::util::profile::PROFILER.with(|p| p.borrow_mut().enter($name));
    };
}

/// Internal representation of scopes as a tree.
struct Scope {
    name: &'static str,

    /// Parent scope in the tree. The root tree has no parent.
    pred: Option<Rc<RefCell<Scope>>>,

    /// Child scope in the tree.
    succs: Vec<Rc<RefCell<Scope>>>,

    /// How often has this scope been visited?
    num_calls: usize,

    /// In total, how much time has been spent in this scope?
    duration_sum: Duration,

    /// At which time was this scope last entered?
    start_instant: Option<Instant>,
}

impl Scope {
    fn new(name: &'static str, pred: Option<Rc<RefCell<Scope>>>) -> Scope {
        Scope {
            name,
            pred,
            succs: Vec::new(),
            num_calls: 0,
            start_instant: None,
            duration_sum: Duration::new(0, 0),
        }
    }

    /// Enter this scope. Returns a `Guard` instance that should be dropped
    /// when leaving the scope.
    fn enter(&mut self) -> Guard {
        self.num_calls += 1;
        self.start_instant = Some(Instant::now());
        Guard
    }

    /// Leave this scope. Usually called automatically by the `Guard` instance.
    fn leave(&mut self) {
        self.duration_sum = self
            .duration_sum
            .checked_add(self.start_instant.unwrap().elapsed())
            .unwrap();
    }

    fn print_rec<W: std::io::Write>(&self, out: &mut W, root_duration_sum_secs: f64, depth: usize) {
        let duration_sum_secs = self.duration_sum.as_fractional_secs();
        let percent = self
            .pred
            .clone()
            .map(|pred| duration_sum_secs / pred.borrow().duration_sum.as_fractional_secs())
            .unwrap_or(1.0)
            * 100.0;

        // Write self
        for _ in 0..depth {
            write!(out, "  ").unwrap();
        }
        writeln!(
            out,
            "{}: {:3.2}% {:>4.2}ms/call @ {:.2}Hz",
            self.name,
            percent,
            duration_sum_secs * 1000.0 / (self.num_calls as f64),
            self.num_calls as f64 / root_duration_sum_secs,
        )
        .unwrap();

        // Write children
        for succ in &self.succs {
            succ.borrow()
                .print_rec(out, root_duration_sum_secs, depth + 1);
        }
    }
}

pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {
        PROFILER.with(|p| p.borrow_mut().leave());
    }
}

/// A `Profiler` stores the scope tree and keeps track of the currently active
/// scope.
///
/// A `Profiler` has exactly one root scope, which must be maintained by
/// calling `Profiler::start_frame` at the start of each frame and dropping
/// the resulting `Guard` only at the end of the frame.
///
/// Note that there is a global instance of `Profiler` in `PROFILER`, so it is
/// not possible to manually create an instance of `Profiler`.
pub struct Profiler {
    root: Rc<RefCell<Scope>>,
    current: Rc<RefCell<Scope>>,
}

impl Profiler {
    fn new() -> Profiler {
        let root = Rc::new(RefCell::new(Scope::new("root", None)));

        Profiler {
            root: root.clone(),
            current: root,
        }
    }

    /// This method must be called once at the start of each frame. The
    /// resulting `Guard` must be kept alive until the end of the frame.
    fn start_frame(&mut self) -> Guard {
        assert!(
            self.current.borrow().pred.is_none(),
            "should start frame at root profiling node"
        );

        // This enables easily resetting profiling data in `reset`.
        self.current = self.root.clone();

        let mut current = self.current.borrow_mut();
        current.enter()
    }

    /// Enter a scope. Returns a `Guard` that should be dropped upon leaving
    /// the scope.
    ///
    /// Usually, this method will be called by the `profile!` macro, so it does
    /// not need to be used directly.
    pub fn enter(&mut self, name: &'static str) -> Guard {
        // Does the current scope already have `name` as a successor?
        let existing_succ = {
            let current = self.current.borrow();

            current
                .succs
                .iter()
                .find(|succ| succ.borrow().name == name)
                .cloned()
        };

        let succ = if let Some(existing_succ) = existing_succ {
            existing_succ
        } else {
            // Add new successor to current scope.
            let succ = Rc::new(RefCell::new(Scope::new(name, Some(self.current.clone()))));
            self.current.borrow_mut().succs.push(succ.clone());
            succ
        };

        self.current = succ;
        self.current.borrow_mut().enter()
    }

    /// Completely reset profiling data.
    pub fn reset(&mut self) {
        self.root = Rc::new(RefCell::new(Scope::new("root", None)));
    }

    /// Leave the current scope.
    fn leave(&mut self) {
        self.current.borrow_mut().leave();

        // Set current scope back to the parent node.
        if self.current.borrow().pred.is_some() {
            let pred = self.current.borrow().pred.clone().unwrap();
            self.current = pred;
        }
    }

    fn print<W: std::io::Write>(&self, out: &mut W) {
        let root_duration_sum_secs = self.root.borrow().duration_sum.as_fractional_secs();

        self.root.borrow().print_rec(out, root_duration_sum_secs, 0);

        out.flush().unwrap();
    }
}
