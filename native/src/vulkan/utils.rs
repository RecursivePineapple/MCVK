use std::{
    cell::RefCell,
    sync::{Arc, RwLock},
};

use smallvec::SmallVec;

pub type Ref<T> = Arc<RefCell<T>>;

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
