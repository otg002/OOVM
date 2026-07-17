use std::collections::HashMap;
use std::rc::Rc;

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub enum Segment {
    Bytes(Vec<u8>),
    Fields(Vec<Data>),
}

#[derive(Debug)]
pub struct Memory {
    pub mem: HashMap<u32, (usize, Rc<String>, Segment)>,
    next_ptr: u32,
}
impl Memory {
    pub fn new() -> Memory {
        Memory {
            mem: HashMap::new(),
            next_ptr: 0,
        }
    }
    pub fn alloc(&mut self, type_name: TypeName, class: Type) -> u32 {
        let ptr = self.next_ptr;
        self.next_ptr += 1;
        while self.mem.contains_key(&self.next_ptr) {
            self.next_ptr += 1;
        }
        let value: Segment = if class.object {
            let mut fields: Vec<Data> = Vec::with_capacity(class.size);
            for _ in 0..class.size {
                fields.push(Data::Value(0))
            }
            Segment::Fields(fields)
        } else {
            let mut bytes: Vec<u8> = Vec::with_capacity(class.size);
            for _ in 0..class.size {
                bytes.push(0);
            }
            Segment::Bytes(bytes)
        };
        self.mem.insert(ptr, (1, type_name.into(), value));
        ptr
    }
    pub fn alloc_blank(&mut self, type_name: TypeName) -> u32 {
        let ptr = self.next_ptr;
        self.next_ptr += 1;
        while self.mem.contains_key(&self.next_ptr) {
            self.next_ptr += 1;
        }
        self.mem.insert(ptr, (1, Rc::new(type_name), Segment::Bytes(Vec::new())));
        ptr
    }
    pub fn reference(&mut self, ptr: u32) -> Result<()> {
        self.mem.get_mut(&ptr).ok_or(Error::UnknownPointer(ptr))?.0 += 1;
        Ok(())
    }
    pub fn free(&mut self, ptr: u32) -> Result<()> {
        let ref_count: &mut usize = &mut self.mem.get_mut(&ptr).ok_or(Error::UnknownPointer(ptr))?.0;
        if *ref_count == 0 {
            return Ok(());
        }
        *ref_count -= 1;
        if *ref_count == 0 {
            match self.mem.get(&ptr).ok_or(Error::UnknownPointer(ptr))?.2.clone() {
                Segment::Fields(fields) => {
                    for field in fields {
                        if let Data::Pointer(field) = field {
                            self.free(field)?;
                        }
                    }
                }
                Segment::Bytes(_) => {}
            }
            self.mem.remove(&ptr);
        }
        Ok(())
    }
    pub fn get_type<'a>(&self, vm: &'a Vm, ptr: u32) -> Result<(&'a Type, TypeName)> {
        let type_name = self.mem.get(&ptr).ok_or(Error::UnknownPointer(ptr))?.1.clone();
        let type_ = vm.types.get(&type_name.to_string()).ok_or(Error::TypeNotFound(type_name.to_string()));
        Ok((type_?, type_name.to_string()))
    }
    pub fn write_data(&mut self, ptr: u32, val: Data, index: usize) -> Result<()> {
        let mem: &mut Segment = &mut self.mem.get_mut(&ptr).ok_or(Error::UnknownPointer(ptr))?.2;
        match *mem {
            Segment::Fields(ref mut fields) => {
                let len = fields.len();
                let field = fields.get_mut(index).ok_or(Error::IndexOutOfBounds(len, index))?;
                *field = val;
            }
            Segment::Bytes(ref mut bytes) => {
                let len = bytes.len();
                for (i, byte) in val.to_bytes().iter().enumerate() {
                    let b = bytes.get_mut(i + index).ok_or(Error::IndexOutOfBounds(len, index))?;
                    *b = *byte;
                }
            }
        }
        Ok(())
    }
    pub fn write_all(&mut self, ptr: u32, seg: Segment) -> Result<()> {
        let mem: &mut (usize, Rc<String>, Segment) = self.mem.get_mut(&ptr).ok_or(Error::UnknownPointer(ptr))?;
        mem.2 = seg;
        Ok(())
    }
    pub fn read(&self, ptr: u32, index: usize) -> Result<Data> {
        let seg = &self.mem.get(&ptr).ok_or(Error::UnknownPointer(ptr))?.2;
        Ok(match *seg {
            Segment::Fields(ref fields) => {
                fields.get(index).ok_or(Error::IndexOutOfBounds(fields.len(), index))?.clone()
            }
            Segment::Bytes(ref bytes) => {
                if index + 4 >= bytes.len() {
                    return Err(Error::IndexOutOfBounds(bytes.len(), index));
                }
                Value(bytes_to_u32(&bytes[index..index+4]))
            }
        })
    }
}