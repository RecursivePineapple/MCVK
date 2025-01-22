use std::cell::RefCell;
use std::hash::Hash;
use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::RwLock;

use num::Num;
use smallvec::SmallVec;

#[derive(Debug)]
pub struct Ref<T>(Arc<RwLock<T>>)
where
    T: Send + Sync,
    Self: Send + Sync;

impl<T: Send + Sync> Clone for Ref<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Send + Sync> Ref<T> {
    pub fn new(value: T) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }

    pub fn read(&self) -> impl Deref<Target = T> + '_ {
        self.0.read().unwrap()
    }

    pub fn write(&self) -> impl DerefMut<Target = T> + '_ {
        self.0.write().unwrap()
    }
}

/// This type should ONLY be accessed from the main render thread.
/// Access on other threads is undefined behaviour.
pub struct MainRenderThread<T>(pub T);

unsafe impl<T> Sync for MainRenderThread<T> {}
unsafe impl<T> Send for MainRenderThread<T> {}

pub trait Extract: Default {
    fn extract(&mut self) -> Self;
}

impl<T: Default> Extract for T {
    fn extract(&mut self) -> Self {
        std::mem::take(self)
    }
}

#[derive(Debug, Clone)]
pub enum TypedVec {
    F32s(SmallVec<[f32; 1]>),
    F64s(SmallVec<[f64; 1]>),
    I32s(SmallVec<[i32; 1]>),
    U32s(SmallVec<[u32; 1]>),
    I16s(SmallVec<[i16; 1]>),
    U16s(SmallVec<[u16; 1]>),
    I8s(SmallVec<[i8; 1]>),
    U8s(SmallVec<[u8; 1]>),
    I64s(SmallVec<[i64; 1]>),
    U64s(SmallVec<[u64; 1]>),
}

impl From<f32> for TypedVec {
    fn from(value: f32) -> Self {
        Self::F32s(SmallVec::from([value; 1]))
    }
}

impl From<f64> for TypedVec {
    fn from(value: f64) -> Self {
        Self::F64s(SmallVec::from([value; 1]))
    }
}

impl From<i32> for TypedVec {
    fn from(value: i32) -> Self {
        Self::I32s(SmallVec::from([value; 1]))
    }
}

impl From<u32> for TypedVec {
    fn from(value: u32) -> Self {
        Self::U32s(SmallVec::from([value; 1]))
    }
}

impl From<i16> for TypedVec {
    fn from(value: i16) -> Self {
        Self::I16s(SmallVec::from([value; 1]))
    }
}

impl From<u16> for TypedVec {
    fn from(value: u16) -> Self {
        Self::U16s(SmallVec::from([value; 1]))
    }
}

impl From<i8> for TypedVec {
    fn from(value: i8) -> Self {
        Self::I8s(SmallVec::from([value; 1]))
    }
}

impl From<u8> for TypedVec {
    fn from(value: u8) -> Self {
        Self::U8s(SmallVec::from([value; 1]))
    }
}

impl From<i64> for TypedVec {
    fn from(value: i64) -> Self {
        Self::I64s(SmallVec::from([value; 1]))
    }
}

impl From<u64> for TypedVec {
    fn from(value: u64) -> Self {
        Self::U64s(SmallVec::from([value; 1]))
    }
}

pub fn map<T: Num + Copy>(x: T, in_min: T, in_max: T, out_min: T, out_max: T) -> T {
    (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min
}

pub struct ArcPtrKey<'a, T>(pub &'a Arc<T>);

impl<T> Hash for ArcPtrKey<'_, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(self.0).hash(state);
    }
}

impl<T> PartialEq for ArcPtrKey<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.0, other.0)
    }
}
impl<T> Eq for ArcPtrKey<'_, T> {}

impl<T> PartialOrd for ArcPtrKey<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Arc::as_ptr(self.0).partial_cmp(&Arc::as_ptr(&other.0))
    }
}
impl<T> Ord for ArcPtrKey<'_, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Arc::as_ptr(self.0).cmp(&Arc::as_ptr(&other.0))
    }
}
