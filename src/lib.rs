#![no_std]

use embedded_hal::serial::Write;
use heapless::LinearMap;
use nb;

pub const NAME_SZ: usize = 24;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ValueType {
    Bool = 0,
    Int = 1,
    Float = 2,
}

#[derive(Clone)]
pub struct ValueRec<const P: usize> {
    pub is_active: bool,
    pub is_only_front: bool,
    pub vtype: ValueType,
    pub vals: [i32; P],
}

impl<const P: usize> ValueRec<P> {
    pub const fn new(vtype: ValueType) -> Self {
        Self {
            is_active: false,
            is_only_front: false,
            vtype,
            vals: [0; P],
        }
    }
}

pub enum AddError {
    MapOverflow,
}

#[derive(Clone, Copy)]
pub struct SVstruct<M> {
    current: usize,
    pub map: M,
}

pub type SV<const N: usize, const P: usize> = SVstruct<LinearMap<&'static str, ValueRec<P>, N>>;

pub trait AddValue {
    fn add_bool_value(
        &mut self,
        name: &'static str,
        value_in: bool,
        only_pos_front: bool,
    ) -> Result<(), AddError> {
        Ok(self.add_value(name, ValueType::Bool, i32::from(value_in), only_pos_front)?)
    }

    fn add_int_value(&mut self, name: &'static str, value_in: i32) -> Result<(), AddError> {
        Ok(self.add_value(name, ValueType::Int, value_in, false)?)
    }

    fn add_float_value(&mut self, name: &'static str, value_in: f32) -> Result<(), AddError> {
        Ok(self.add_value(name, ValueType::Float, value_in.to_bits() as i32, false)?)
    }

    fn add_value(
        &mut self,
        name: &'static str,
        vtype: ValueType,
        val: i32,
        only_pos_front: bool,
    ) -> Result<(), AddError>;
}

impl<const N: usize, const P: usize> SV<N, P> {
    pub const fn new() -> Self {
        Self {
            current: 0,
            map: LinearMap::new(),
        }
    }

    pub fn next<F>(&mut self, f: F)
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
            v.vals[self.current] = v.vals[previous]
        }
    }
}

impl<const N: usize, const P: usize> AddValue for SV<N, P> {
    fn add_value(
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
        vr.is_active = true;
        vr.is_only_front = only_pos_front;

        Ok(())
    }
}

pub trait SendPackage<V> {
    type Error;
    fn send_package(&mut self, module: &'static str, values: &V) -> Result<(), Self::Error>;
}

impl<Tx, const N: usize, const P: usize> SendPackage<SV<N, P>> for Tx
where
    Tx: Write<u8>,
{
    type Error = nb::Error<<Tx as Write<u8>>::Error>;
    fn send_package(&mut self, module: &'static str, values: &SV<N, P>) -> Result<(), Self::Error> {
        //if values.map.is_empty() {
        //    return Err(Error::EmptyPackage);
        //}

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
            // Data for single variable
            for b in k.bytes() {
                self.write(b)?;
            }
            for _ in 0..NAME_SZ - k.len() {
                self.write(0)?;
            }

            for &b in &(v.vtype as i32).to_le_bytes() {
                self.write(b)?;
            }

            for val in &v.vals {
                for &b in &val.to_le_bytes() {
                    self.write(b)?;
                }
            }
        }

        for &b in b"=end=" {
            self.write(b)?;
        }

        Ok(())
    }
}
