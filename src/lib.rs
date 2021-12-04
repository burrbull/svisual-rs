//! Embedded client of [SVisual](https://github.com/Tyill/SVisual/) monitor
//!
//! Requires Rust 1.51

#![no_std]
#![deny(missing_docs)]

/// Prelude module for easy import
pub mod prelude;

use embedded_hal::serial::Write;
use heapless::LinearMap;
use nb;

/// Boolean signal that shows only positive front impulses
pub struct OnlyFront(pub bool);

/// Types supported by SVisual
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ValueType {
    /// Boolean value
    Bool = 0,
    /// `i32` value
    Int = 1,
    /// `f32` value
    Float = 2,
}

/// Value Record. Contents values of 1 signal. `P` is package size
#[derive(Clone)]
pub struct ValueRec<const P: usize> {
    /// Only positive front
    is_only_front: bool,
    vtype: ValueType,
    vals: [i32; P],
}

impl<const P: usize> ValueRec<P> {
    /// Create empty Value Record
    pub const fn new(vtype: ValueType) -> Self {
        Self {
            is_only_front: false,
            vtype,
            vals: [0; P],
        }
    }
}

/// Errors of adding values to container
pub enum AddError {
    /// Overflow of container
    MapOverflow,
}

/// Go to next sendable value position
pub trait NextValue {
    /// Use previous values if no update will come.
    /// `F` is send package function
    fn next<F>(&mut self, f: F)
    where
        F: FnOnce(&Self);
}

/// Generic signal container
#[derive(Clone, Copy)]
pub struct SVStruct<M> {
    current: usize,
    map: M,
}

impl<M> core::ops::Deref for SVStruct<M> {
    type Target = M;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl<M> core::ops::DerefMut for SVStruct<M> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.map
    }
}

/// Map of signals
pub type SVMap<const N: usize, const P: usize> = SVStruct<LinearMap<&'static str, ValueRec<P>, N>>;

impl<const N: usize, const P: usize> SVMap<N, P> {
    /// Create new instance
    pub const fn new() -> Self {
        Self {
            current: 0,
            map: LinearMap::new(),
        }
    }

    /// Checks if package is empty
    pub fn is_first(&self) -> bool {
        self.current == 0
    }

    /// Checks if package is full
    pub fn is_last(&self) -> bool {
        self.current == P - 1
    }

    fn set_value(
        &mut self,
        name: &'static str,
        vtype: ValueType,
        val: i32,
        only_pos_front: bool,
    ) -> Result<(), AddError> {
        if !self.map.contains_key(&name) {
            if self.map.insert(name, ValueRec::new(vtype)).is_err() {
                return Err(AddError::MapOverflow);
            }
        }

        let vr = self.map.get_mut(name).unwrap();
        vr.vals[self.current] = val;
        vr.is_only_front = only_pos_front;

        Ok(())
    }

    /// Update value of specified type at current time position
    pub fn set<T: Value>(&mut self, name: &'static Name, value: T) -> Result<(), AddError> {
        self.set_value(name, T::TYPE, value.to_i32(), T::ONLY_FRONT)
    }
}

/// Supported value transfer type
pub trait Value {
    /// Associated `[ValueType]`
    const TYPE: ValueType;
    /// Only positive front
    const ONLY_FRONT: bool;
    /// `i32` representation
    fn to_i32(self) -> i32;
}

impl Value for i32 {
    const TYPE: ValueType = ValueType::Int;
    const ONLY_FRONT: bool = false;
    fn to_i32(self) -> i32 {
        self
    }
}

impl Value for f32 {
    const TYPE: ValueType = ValueType::Float;
    const ONLY_FRONT: bool = false;
    fn to_i32(self) -> i32 {
        self.to_bits() as i32
    }
}

impl Value for bool {
    const TYPE: ValueType = ValueType::Bool;
    const ONLY_FRONT: bool = false;
    fn to_i32(self) -> i32 {
        self as i32
    }
}

impl Value for OnlyFront {
    const TYPE: ValueType = ValueType::Bool;
    const ONLY_FRONT: bool = false;
    fn to_i32(self) -> i32 {
        self.0 as i32
    }
}

impl<const N: usize, const P: usize> NextValue for SVMap<N, P> {
    fn next<F>(&mut self, f: F)
    where
        F: FnOnce(&Self),
    {
        let previous = self.current;
        self.current += 1;
        if self.current >= P {
            self.current -= P;
            f(self);
        }
        for (_, v) in self.map.iter_mut() {
            v.vals[self.current] = if v.is_only_front { 0 } else { v.vals[previous] };
        }
    }
}

/// Form and send package
pub trait SendPackage<V> {
    /// Error type
    type Error;
    /// Send package with module name
    fn send_package(&mut self, module: &'static Name, values: &V) -> Result<(), Self::Error>;
}

/// Implementation of SendPackage for all that support `embedded-hal::serial::Write`
impl<Tx, const N: usize, const P: usize> SendPackage<SVMap<N, P>> for Tx
where
    Tx: WriteIter,
{
    type Error = <Tx as WriteIter>::Error;
    fn send_package(
        &mut self,
        module: &'static Name,
        values: &SVMap<N, P>,
    ) -> Result<(), Self::Error> {
        use core::iter::repeat;
        let vl_size = Name::MAX_SIZE + 4 + P * 4;
        // Full package size
        let full_size = (Name::MAX_SIZE + vl_size * values.map.len()) as u32;

        // Open package
        self.bwrite_iter(
            "=begin="
                .bytes()
                .chain(full_size.to_le_bytes().iter().cloned())
                // Identifier (name) of the module
                .chain(module.bytes())
                .chain(repeat(0).take(Name::MAX_SIZE - module.len())),
        )?;
        self.bflush()?;

        for (&name, v) in values.map.iter() {
            // Identifier (name) of signal
            self.bwrite_iter(
                name.bytes()
                    .chain(repeat(0).take(Name::MAX_SIZE - name.len()))
                    // Signal type
                    .chain((v.vtype as i32).to_le_bytes().iter().cloned())
                    // Values of one signal in package
                    .chain(v.vals.iter().flat_map(|val| val.to_le_bytes())),
            )?;
            self.bflush()?;
        }

        // Close package
        self.bwrite_iter("=end=".bytes())?;
        self.bflush()?;

        Ok(())
    }
}

/// Compile-time chacked name string
pub struct Name(&'static str);

impl core::ops::Deref for Name {
    type Target = str;
    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl Name {
    /// Maximum length of module/signal name
    const MAX_SIZE: usize = 24;

    /// New name instance
    pub const fn new(name: &'static str) -> Self {
        assert!(!name.is_empty());
        assert!(name.len() < Self::MAX_SIZE);
        assert!(!equal(name, "=end="));
        assert!(!equal(name, "=begin="));
        Self(name)
    }
}

const fn equal(first: &'static str, second: &'static str) -> bool {
    if first.len() != second.len() {
        return false;
    }
    let fb = first.as_bytes();
    let sb = second.as_bytes();
    let mut i = 0;
    while i < first.len() {
        if fb[i] != sb[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Write iterator
pub trait WriteIter {
    /// Error type
    type Error;
    /// Blocking write of iterator
    fn bwrite_iter<WI>(&mut self, bytes: WI) -> Result<(), Self::Error>
    where
        WI: Iterator<Item = u8>;
    /// Blocking flush
    fn bflush(&mut self) -> Result<(), Self::Error>;
}

impl<Tx> WriteIter for Tx
where
    Tx: Write<u8>,
{
    type Error = <Tx as Write<u8>>::Error;

    fn bwrite_iter<WI>(&mut self, mut bytes: WI) -> Result<(), Self::Error>
    where
        WI: Iterator<Item = u8>,
    {
        bytes.try_for_each(|c| nb::block!(self.write(c)))
    }

    fn bflush(&mut self) -> Result<(), Self::Error> {
        nb::block!(self.flush())
    }
}
