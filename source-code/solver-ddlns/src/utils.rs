//! Ici je mets juste un peu de code qui n'est pas directement lié au projet
//! mais qui s'avère quand même être utile pour la résolution du probleme.

use std::{
    alloc::{GlobalAlloc, System},
    fmt::{Debug, Display},
    ops::{Index, IndexMut},
    sync::atomic::{AtomicUsize, Ordering},
};

use signal_hook::{consts::SIGINT, low_level::raise};

/// This structure implements a 2D matrix of size [ n X m ].
///
///
/// # Example
/// ```
/// # use psp-parsing::Matrix;
///
/// let mut adjacency = Matrix::new_default(5, 5, None);
///
/// adjacency[(2, 2)] = Some(-5);
/// assert_eq!(Some(-5), adjacency[(2, 2)]);
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Matrix<T> {
    /// The number of rows
    pub n: usize,
    /// The number of columns
    pub m: usize,
    /// The items of the matrix
    pub data: Vec<T>,
}
impl<T: Default + Clone> Matrix<T> {
    /// Allows the creation of a matrix initialized with the default element
    pub fn new(n: usize, m: usize) -> Self {
        Matrix {
            n,
            m,
            data: vec![Default::default(); m * n],
        }
    }

    pub fn row(&self, i: usize) -> impl Iterator<Item = &T> {
        let start = self.pos((i, 0));
        let end = self.pos((i, self.m));
        self.data[start..end].iter()
    }
    pub fn row_mut(&mut self, i: usize) -> impl Iterator<Item = &mut T> {
        let start = self.pos((i, 0));
        let end = self.pos((i, self.m));
        self.data[start..end].iter_mut()
    }
    pub fn col(&self, i: usize) -> impl Iterator<Item = &T> {
        (0..self.n).map(move |r| &self.data[self.pos((r, i))])
    }
    pub fn col_mut(&mut self, i: usize) -> impl Iterator<Item = &mut T> {
        let ptr = self.data.as_mut_ptr();
        (0..self.n).map(move |r| unsafe { ptr.add(self.pos((r, i))).as_mut().unwrap() })
    }
}
impl<T: Clone> Matrix<T> {
    /// Allows the creation of a matrix initialized with the default element
    pub fn new_default(m: usize, n: usize, item: T) -> Self {
        Matrix {
            m,
            n,
            data: vec![item; m * n],
        }
    }
}
impl<T> Matrix<T> {
    /// Returns the position (offset in the data) of the given index
    fn pos(&self, idx: (usize, usize)) -> usize {
        debug_assert!(idx.0 < self.n, "position invalide: m");
        debug_assert!(
            idx.1 < self.m,
            "position invalide: {} >= n {}",
            idx.1,
            self.n
        );
        self.m * idx.0 + idx.1
    }
}
/// A matrix is typically an item you'll want to adress using 2D position
impl<T> Index<(usize, usize)> for Matrix<T> {
    type Output = T;

    /// It returns a reference to some item from the matrix at the given 2D index
    fn index(&self, idx: (usize, usize)) -> &Self::Output {
        let position = self.pos(idx);
        &self.data[position]
    }
}
impl<T> IndexMut<(usize, usize)> for Matrix<T> {
    /// It returns a mutable reference to some item from the matrix at the given 2D index
    fn index_mut(&mut self, idx: (usize, usize)) -> &mut Self::Output {
        let position = self.pos(idx);
        &mut self.data[position]
    }
}

/// Tell the compiler how to visually display the matrix when in debug mode
impl<T: Debug> Debug for Matrix<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, v) in self.data.iter().enumerate() {
            if i % self.m == 0 {
                writeln!(f)?;
            }
            write!(f, " {:>5?}", v)?;
        }
        writeln!(f)
    }
}

/// Tell the compiler how to visually display the matrix
impl<T: Display> Display for Matrix<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, v) in self.data.iter().enumerate() {
            if i % self.m == 0 {
                writeln!(f)?;
            }
            write!(f, " {:>5}", v)?;
        }
        writeln!(f)
    }
}

// ----------------------------------------------------------------------------
/// Un allocateur qui envoie un signal lorsqu'une limite de mémoire a été
/// allouée.
// ----------------------------------------------------------------------------
pub struct SigLimitAllocator<A = System> {
    /// The max number of bytes that may be allocated
    limit: AtomicUsize,
    /// The current amount of ram which is being currently allocated
    used: AtomicUsize,
    /// Some bookkeeping to keep track of the max amount of ram which has ever
    /// been allocated
    peak: AtomicUsize,
    /// The global allocator we delegate the bulk of the work to (by default,
    /// the system allocator)
    alloc: A,
}
#[allow(dead_code)]
impl<A> SigLimitAllocator<A> {
    pub const fn new(alloc: A, limit: usize) -> Self {
        Self {
            limit: AtomicUsize::new(limit),
            used: AtomicUsize::new(0),
            peak: AtomicUsize::new(0),
            alloc,
        }
    }

    pub fn set_limit_gb(&self, limit: f64) {
        self.set_limit_mb(1024.0 * limit)
    }
    pub fn set_limit_mb(&self, limit: f64) {
        self.set_limit_kb(limit * 1024.0)
    }
    pub fn set_limit_kb(&self, limit: f64) {
        self.set_limit((limit * 1024.0).ceil() as usize)
    }
    pub fn set_limit(&self, limit: usize) {
        self.limit.store(limit, Ordering::Relaxed)
    }
    pub fn get_usage(&self) -> usize {
        self.used.load(Ordering::Relaxed)
    }
    pub fn get_usage_kb(&self) -> f64 {
        self.get_usage() as f64 / 1024.0
    }
    pub fn get_usage_mb(&self) -> f64 {
        self.get_usage_kb() / 1024.0
    }
    pub fn get_usage_gb(&self) -> f64 {
        self.get_usage_mb() / 1024.0
    }
    pub fn get_peak(&self) -> usize {
        self.peak.load(Ordering::Relaxed)
    }
    pub fn get_peak_kb(&self) -> f64 {
        self.get_peak() as f64 / 1024.0
    }
    pub fn get_peak_mb(&self) -> f64 {
        self.get_peak_kb() / 1024.0
    }
    pub fn get_peak_gb(&self) -> f64 {
        self.get_peak_mb() / 1024.0
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for SigLimitAllocator<A> {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        // fetch_add returns the previous value
        // fetch_max returns the previous value
        let size = layout.size();
        let old_used = self.used.fetch_add(size, Ordering::SeqCst);
        let used = old_used + size;

        if old_used > self.limit.load(Ordering::Relaxed) {
            // It is not allowed to fail during an allocation.
            // Thus, we simply ignore the potential problems that might cause
            // the system's inability to send the signal; and we assume that
            // it will be rethrown some time later
            let _ = raise(SIGINT);
        }

        // keep track of the max quantity of allocated memory
        self.peak.fetch_max(used, Ordering::SeqCst);

        // actually proceed to allocation
        self.alloc.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: std::alloc::Layout) {
        self.used.fetch_sub(layout.size(), Ordering::SeqCst);
        self.alloc.dealloc(ptr, layout)
    }
}
