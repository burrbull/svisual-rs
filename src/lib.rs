#![no_std]

pub const NAME_SZ : usize = 24;
pub const PACKET_SZ : usize = 10;
pub const VL_SZ : usize = (NAME_SZ+4+PACKET_SZ*4);

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum ValueType {
    Bool  = 0,
    Int   = 1,
    Float = 2,
}

pub struct ValueRec {
    pub is_active     : bool,
    pub is_only_front : bool,
    pub vtype : ValueType,
    pub vals : heapless::Vec<i32, heapless::consts::U10>,
}

impl ValueRec {
    pub fn new(vtype: ValueType) -> ValueRec {
        ValueRec {
            is_active: false,
            is_only_front: false,
            vtype,
            vals: heapless::Vec::new(),
        }
    }
}

pub enum AddError {
    MapOverflow,
    PackageOverflow,
    BadName,
}

use core::i32;

pub trait AddValue {
    fn add_bool_value (&mut self, name: &'static [u8], value_in: bool, only_pos_front: bool
    ) -> Result<(), AddError> {
        Ok(self.add_value(name, ValueType::Bool, i32::from(value_in), only_pos_front)?)
    }
    
    fn add_int_value (&mut self, name: &'static [u8], value_in: i32
    ) -> Result<(), AddError> {
        Ok(self.add_value(name, ValueType::Int, value_in, false)?)
    }
    
    fn add_float_value (&mut self, name: &'static [u8], value_in: f32
    ) -> Result<(), AddError> {
        Ok(self.add_value(name, ValueType::Float, value_in.to_bits() as i32, false)?)
    }
    
    fn add_value (&mut self, name: &'static [u8], vtype: ValueType, val: i32, only_pos_front: bool
    ) -> Result<(), AddError>;
    
    fn clear_data (&mut self);
}

macro_rules! impl_add_value {
    ($U:ty) => {
        impl AddValue for heapless::FnvIndexMap<&[u8], ValueRec, $U> {
            fn add_value(
                &mut self,
                name: &'static [u8],
                vtype: ValueType,
                val: i32,
                only_pos_front: bool
            ) -> Result<(), AddError> {
                if !self.contains_key(name) {
                    let len = name.len();
                    if (len == 0) || (len >= NAME_SZ) ||
                    (name == b"=end=") || (name == b"=begin=") {
                        return Err(AddError::BadName);
                    }
                    if self.insert(name, ValueRec::new(vtype)).is_err() {
                        return Err(AddError::MapOverflow);
                    }
                }
                
                let vr = self.get_mut(name).unwrap();
                if vr.vals.push(val).is_err() {
                    return Err(AddError::PackageOverflow);
                }
                vr.is_active = true;
                vr.is_only_front = only_pos_front;
                
                Ok(())
            }
            
            fn clear_data (&mut self) {
                for (_, v) in self {
                    v.vals.clear()
                }
            }
        }
    };
}

use heapless::consts::{U1, U2, U4, U8, U16, U32, U64, U128};
impl_add_value!(U1);
impl_add_value!(U2);
impl_add_value!(U4);
impl_add_value!(U8);
impl_add_value!(U16);
impl_add_value!(U32);
impl_add_value!(U64);
impl_add_value!(U128);
