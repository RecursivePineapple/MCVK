use std::{
    cell::UnsafeCell,
    fmt::Debug,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

pub struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock<'a>(&'a self) -> SpinGuard<'a, T> {
        while self.locked.swap(true, Ordering::Acquire) {
            spin_loop();
        }
        SpinGuard { lock: self }
    }
}

unsafe impl<T> Send for SpinLock<T> {}
unsafe impl<T> Sync for SpinLock<T> {}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

impl<T: Debug> Debug for SpinGuard<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.deref().fmt(f)
    }
}

impl<T: Debug> Debug for SpinLock<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.lock().fmt(f)
    }
}

impl<T> SpinLock<Arc<T>> {
    pub fn get(&self) -> Arc<T> {
        Arc::clone(&*self.lock())
    }

    pub fn set(&self, new: Arc<T>) {
        *self.lock() = new;
    }
}

impl<T> SpinLock<Vec<T>> {
    pub fn push(&self, value: T) {
        self.lock().push(value);
    }
}

impl<T> SpinLock<T> {
    pub fn swap(&self, mut new: T) -> T {
        let mut l = self.lock();
        std::mem::swap(&mut new, &mut l);
        new
    }
}

impl<T: Default> SpinLock<T> {
    pub fn extract(&self) -> T {
        std::mem::take(&mut *self.lock())
    }
}
