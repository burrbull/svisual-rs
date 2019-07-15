#![no_std]

use core::i32;

use heapless::LinearMap;
use generic_array::GenericArray;
use generic_array::sequence::GenericSequence;
use typenum::marker_traits::PowerOfTwo;

pub const NAME_SZ : usize = 24;

#[derive(Clone,Copy,Debug,PartialEq)]
pub enum ValueType {
    Bool  = 0,
    Int   = 1,
    Float = 2,
}

#[derive(Clone)]
pub struct ValueRec<P> where P: generic_array::ArrayLength<i32> {
    pub is_active     : bool,
    pub is_only_front : bool,
    pub vtype : ValueType,
    pub vals : GenericArray<i32, P>,
}

impl<P> ValueRec<P> where P: generic_array::ArrayLength<i32> {
    pub fn new(vtype: ValueType) -> ValueRec<P> {
        ValueRec {
            is_active: false,
            is_only_front: false,
            vtype,
            vals: GenericArray::generate(|_| {0i32})
        }
    }
}

pub enum AddError {
    MapOverflow,
}

#[derive(Clone,Copy)]
pub struct SVstruct<M> {
    current: usize,
    pub map: M
}

pub type SV<N, P> = SVstruct<LinearMap<&'static [u8], ValueRec<P>, N>>;

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
}

impl<N, P> SV<N, P>
where
    N: heapless::ArrayLength<(&'static [u8], ValueRec<P>)> + PowerOfTwo,
    P: generic_array::ArrayLength<i32> + typenum::marker_traits::Unsigned
{
    pub fn new() -> Self {
        Self {
            current: 0,
            map: LinearMap::new()
        }
    }
    
    pub fn next<F>(&mut self, f: F)
    where
        F: FnOnce(&Self) {
        let previous = self.current;
        self.current += 1;
        let packet_size = P::to_usize();
        if self.current >= packet_size {
            self.current -= packet_size;
            f(self);
        }
        for (_, v) in self.map.iter_mut() {
            v.vals[self.current] = v.vals[previous]
        }
    }
}

impl<N, P> AddValue for SV<N, P>
where
    N: heapless::ArrayLength<(&'static [u8], ValueRec<P>)>,
    P: generic_array::ArrayLength<i32>
{
    fn add_value(
        &mut self,
        name: &'static [u8],
        vtype: ValueType,
        val: i32,
        only_pos_front: bool
    ) -> Result<(), AddError> {
        if !self.map.contains_key(&name) {
            debug_assert!((name.len() > 0) && (name.len() <= NAME_SZ));
            debug_assert!((name != b"=end=") && (name != b"=begin="));
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
