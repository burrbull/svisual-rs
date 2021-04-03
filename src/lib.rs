//! Embedded client of [SVisual](https://github.com/Tyill/SVisual/) monitor
//!
//! Requires Rust 1.51

#![no_std]
#![deny(missing_docs)]

use embedded_hal::serial::Write;
use heapless::LinearMap;
use nb;

/// Maximum length of module/signal name
pub const NAME_SZ: usize = 24;

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

/// Filling Record
pub trait SetValue<T> {
    /// Update value of specified type at current time position
    fn set(&mut self, name: &'static str, value: T) -> Result<(), AddError>;
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
    fn set_value(
        &mut self,
        name: &'static str,
        vtype: ValueType,
        val: i32,
        only_pos_front: bool,
    ) -> Result<(), AddError> {
        if !self.map.contains_key(&name) {
            debug_assert!((name.len() > 0) && (name.len() <= NAME_SZ));
            debug_assert!((name != "=end=") && (name != "=begin="));
            if self.map.insert(name, ValueRec::new(vtype)).is_err() {
                return Err(AddError::MapOverflow);
            }
        }

        let vr = self.map.get_mut(name).unwrap();
        vr.vals[self.current] = val;
        vr.is_only_front = only_pos_front;

        Ok(())
    }
}

impl<const N: usize, const P: usize> SetValue<i32> for SVMap<N, P> {
    fn set(&mut self, name: &'static str, value: i32) -> Result<(), AddError> {
        self.set_value(name, ValueType::Int, value, false)
    }
}
impl<const N: usize, const P: usize> SetValue<f32> for SVMap<N, P> {
    fn set(&mut self, name: &'static str, value: f32) -> Result<(), AddError> {
        self.set_value(name, ValueType::Float, value as i32, false)
    }
}
impl<const N: usize, const P: usize> SetValue<bool> for SVMap<N, P> {
    fn set(&mut self, name: &'static str, value: bool) -> Result<(), AddError> {
        self.set_value(name, ValueType::Bool, value as i32, false)
    }
}
impl<const N: usize, const P: usize> SetValue<OnlyFront> for SVMap<N, P> {
    fn set(&mut self, name: &'static str, value: OnlyFront) -> Result<(), AddError> {
        self.set_value(name, ValueType::Bool, value.0 as i32, true)
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
    fn send_package(&mut self, module: &'static str, values: &V) -> Result<(), Self::Error>;
}

/// Implementation of SendPackage for all that support `embedded-hal::serial::Write`
impl<Tx, const N: usize, const P: usize> SendPackage<SVMap<N, P>> for Tx
where
    Tx: Write<u8>,
{
    type Error = nb::Error<<Tx as Write<u8>>::Error>;
    fn send_package(
        &mut self,
        module: &'static str,
        values: &SVMap<N, P>,
    ) -> Result<(), Self::Error> {
        // Start send of package
        for &b in b"=begin=" {
            self.write(b)?;
        }

        let vl_size = NAME_SZ + 4 + P * 4;
        // Full package size
        let full_size = (NAME_SZ + vl_size * values.map.len()) as u32;
        for &b in &full_size.to_le_bytes() {
            self.write(b)?;
        }
        // Identifier (name) of the module
        for b in module.bytes() {
            self.write(b)?;
        }
        for _ in 0..NAME_SZ - module.len() {
            self.write(0)?;
        }

        for (&k, v) in values.map.iter() {
            // Identifier (name) of signal
            for b in k.bytes() {
                self.write(b)?;
            }
            for _ in 0..NAME_SZ - k.len() {
                self.write(0)?;
            }

            // Signal type
            for &b in &(v.vtype as i32).to_le_bytes() {
                self.write(b)?;
            }

            // Values of 1 signal in package
            for val in &v.vals {
                for &b in &val.to_le_bytes() {
                    self.write(b)?;
                }
            }
        }

        // Finish send of package
        for &b in b"=end=" {
            self.write(b)?;
        }

        Ok(())
    }
}
