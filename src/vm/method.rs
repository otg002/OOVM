use std::io::Write;
use std::rc::Rc;
use crate::Vm;
use super::*;

#[derive(Debug, Clone)]
pub struct Method {
    version: usize,
    args: u32,
    code: Rc<[u8]>,
}
impl Method {
    pub fn new(version: usize, args: u32, code: Rc<[u8]>) -> Method {
        Self { version, args, code }
    }
    pub fn call(&self, this: u32, vm: &mut Vm, args: Vec<Data>) -> crate::vm::Result<Option<Data>> {
        if vm.debug > 0 {
            println!("Method called.");
        }
        let mut stack: Vec<Data> = Vec::new();
        let mut locals: Vec<Data> = Vec::new();
        for arg in args {
            locals.push(arg);
        }
        let mut i = 0;
        while i < self.code.len() {
            let instruction = self.code[i];
            if vm.debug > 0 {
                println!("\tExecuting instruction {instruction:X}...");
            }
            let _: i32 = match instruction {
                opcodes::operation_2::CONCAT => {
                    let Data::Pointer(arr2) = stack.pop().ok_or(Error::MissingValue(format!("concat")))? else {
                        return Err(Error::ExpectedPointer);
                    };
                    let Data::Pointer(arr1) = stack.pop().ok_or(Error::MissingValue(format!("concat")))? else {
                        return Err(Error::ExpectedPointer);
                    };
                    let (_, type_name, arr1) = vm.mem.mem.get(&arr1).ok_or(Error::UnknownPointer(arr1))?.clone();
                    let arr2 = vm.mem.mem.get(&arr2).ok_or(Error::UnknownPointer(arr2))?.clone().2;
                    match arr1 {
                        memory::Segment::Bytes(mut bytes) => {
                            let memory::Segment::Bytes(other) = arr2 else {
                                return Err(Error::ObjectNotBytes(format!("concat")));
                            };
                            bytes.extend(other);
                            let ptr = vm.mem.alloc_blank(type_name.to_string());
                            vm.mem.write_all(ptr, memory::Segment::Bytes(bytes))?;
                            stack.push(Pointer(ptr));
                        }
                        memory::Segment::Fields(mut fields) => {
                            let memory::Segment::Fields(other) = arr2 else {
                                return Err(Error::BytesNotObject(format!("concat")));
                            };
                            fields.extend(other);
                            let ptr = vm.mem.alloc_blank(type_name.to_string());
                            vm.mem.write_all(ptr, memory::Segment::Fields(fields))?;
                            stack.push(Pointer(ptr));
                        }
                    }
                    i += 1;
                    0
                }
                opcodes::operation_2::MEM_EQUAL => {
                    let ptr1 = stack.pop().ok_or(Error::MissingValue(format!("mem-equal")))?.to_value();
                    let ptr2 = stack.pop().ok_or(Error::MissingValue(format!("mem-equal")))?.to_value();
                    let mem1 = vm.mem.mem.get(&ptr1).ok_or(Error::UnknownPointer(ptr1))?.clone().2;
                    let mem2 = vm.mem.mem.get(&ptr2).ok_or(Error::UnknownPointer(ptr2))?.clone().2;
                    stack.push(Value((mem1 == mem2) as u32));
                    i += 1;
                    0
                }
                opcodes::operation_2::SEND_DYNAMIC => {
                    let msg = stack.pop().ok_or(Error::MissingValue(String::from("send.dyn@message_to_send")))?.to_value();
                    let ptr = stack.pop().ok_or(Error::MissingValue(String::from("send.dyn@object_pointer")))?.to_value();
                    let memory::Segment::Bytes(msg) = vm.mem.mem.get(&msg).ok_or(Error::UnknownPointer(msg))?.2.clone() else {
                        return Err(Error::ObjectNotBytes(String::from("send.dyn@message_to_send")));
                    };
                    let msg = String::try_from(msg).map_err(|_| Error::ConstInterpretationError(String::from("send.dyn@message_to_send is invalid utf-8")))?;
                    let that_type = vm.mem.get_type(vm, ptr)?;
                    let msg = that_type.messages.get(&msg).ok_or(Error::MessageNotFound(msg))?.clone();
                    let mut args: Vec<Data> = Vec::new();
                    let arg_count = msg.args;
                    for _ in 0..arg_count {
                        args.push(stack.pop().ok_or(Error::MissingValue(String::from("arg for `send` operation")))?);
                    }
                    args.reverse();
                    if let Some(ret_val) = msg.call(ptr, vm, args)? {
                        stack.push(ret_val);
                    }
                    i += 1;
                    0
                }
                opcodes::control::RET => {
                    let value = stack.pop();
                    for val in stack {
                        if let Data::Pointer(ptr) = val {
                            vm.mem.free(ptr)?;
                        }
                    }
                    for val in locals {
                        if let Data::Pointer(ptr) = val {
                            vm.mem.free(ptr)?;
                        }
                    }
                    if vm.debug > 0 {
                        println!("Method exiting (returned value)");
                    }
                    return Ok(value);
                }
                opcodes::control::UNLESS => {
                    let predicate: Data = stack.pop().ok_or(Error::MissingValue(String::from("unless")))?;
                    if predicate.to_value() == 0 {
                        let location = bytes_to_u32(&self.code[i+1..i+5]) as usize;
                        i = location;
                    }
                    else {
                        i += 5;
                    }
                    0
                }
                opcodes::control::GOTO => {
                    let location = bytes_to_u32(&self.code[i+1..i+5]) as usize;
                    i = location;
                    0
                }
                opcodes::control::EXIT => {
                    return Err(Error::Panic);
                }
                opcodes::stack::DUP => {
                    stack.push(*stack.last().ok_or(Error::MissingValue(String::from("dup")))?);
                    i += 1;
                    0
                }
                opcodes::stack::DUPN => {
                    let n = bytes_to_u32(&self.code[i + 1..i + 5]) as usize;
                    let mut values: Vec<Data> = Vec::new();
                    for _ in 0..n {
                        values.push(*stack.last().ok_or(Error::MissingValue(String::from("dup")))?);
                    }
                    values.reverse();
                    for _ in 0..2 {
                        for x in values.iter() {
                            stack.push(*x);
                        }
                    }
                    i += 5;
                    0
                }
                opcodes::stack::SWAP => {
                    let a = stack.pop().ok_or(Error::MissingValue(String::from("swap")))?;
                    let b = stack.pop().ok_or(Error::MissingValue(String::from("swap")))?;
                    stack.push(a);
                    stack.push(b);
                    i += 1;
                    0
                }
                opcodes::stack::REV => {
                    let n = bytes_to_u32(&self.code[i + 1..i + 5]) as usize;
                    let mut values: Vec<Data> = Vec::new();
                    for _ in 0..n {
                        values.push(*stack.last().ok_or(Error::MissingValue(String::from("dup")))?);
                    }
                    for x in values.iter() {
                        stack.push(*x);
                    }
                    i += 5;
                    0
                }
                opcodes::mem::NEW => {
                    let const_index = bytes_to_u32(&self.code[i + 1..i + 5]) as usize;
                    let type_name = String::try_from(
                        vm.mem.get_type(vm, this)?
                            .constants[self.version]
                            .get(const_index)
                            .ok_or(
                                Error::ConstInterpretationError(
                                    String::from(
                                        "Type name provided in `new` not found in constant pool"
                                    )))?.to_vec())
                        .map_err(
                            |err| Error::ConstInterpretationError(err.to_string())
                        )?;
                    vm.types.get(&type_name).ok_or(Error::TypeNotFound(type_name.to_string()))?;
                    let ptr: Data = Pointer(vm.mem.alloc(type_name.clone(), vm.types.get(&type_name).ok_or(Error::TypeNotFound(type_name))?.clone()));
                    stack.push(ptr);
                    i += 5;
                    0
                }
                opcodes::mem::FREE => {
                    if let Data::Pointer(ptr) = stack.pop().ok_or(Error::MissingValue(String::from("free")))? {
                        vm.mem.free(ptr)?;
                    }
                    i += 1;
                    0
                }
                opcodes::mem::REF => {
                    if let Data::Pointer(ptr) = stack.last().ok_or(Error::MissingValue(String::from("free")))?.clone() {
                        vm.mem.reference(ptr)?;
                    };
                    i += 1;
                    0
                }
                opcodes::mem::SET => {
                    let field_id = bytes_to_u32(&self.code[i + 1..i + 5]) as usize;
                    let this_type = vm.mem.get_type(vm, this)?;
                    let offset = this_type.field_table
                        .get(self.version)
                        .ok_or(Error::VersionError(
                            format!("method version: {}; max type version: {}",
                                    self.version,
                                    this_type.field_table.len() - 1))
                        )?.clone();
                    let val = stack.pop().ok_or(Error::MissingValue(String::from("set")))?;
                    vm.mem.write_data(this, val, field_id + offset)?;
                    i += 5;
                    0
                }
                opcodes::mem::GET => {
                    let field_id = bytes_to_u32(&self.code[i + 1..i + 5]) as usize;
                    let this_type = vm.mem.get_type(vm, this)?;
                    let offset = this_type.field_table
                        .get(self.version)
                        .ok_or(Error::VersionError(
                            format!("method version: {}; max type version: {}",
                                    self.version,
                                    this_type.field_table.len() - 1))
                        )?.clone();
                    stack.push(vm.mem.read(this, field_id + offset)?);
                    i += 5;
                    0
                }
                opcodes::mem::MAIN => {
                    stack.push(Pointer(0));
                    i += 1;
                    0
                }
                opcodes::mem::THIS => {
                    stack.push(Pointer(this));
                    i += 1;
                    0
                }
                opcodes::mem::GETAT => {
                    let field_id = stack.pop().ok_or(Error::MissingValue(String::from("getat")))?.to_value() as usize;
                    let this_type = vm.mem.get_type(vm, this)?;
                    let offset = this_type.field_table
                        .get(self.version)
                        .ok_or(Error::VersionError(
                            format!("method version: {}; max type version: {}",
                                    self.version,
                                    this_type.field_table.len() - 1))
                        )?.clone();
                    stack.push(vm.mem.read(this, field_id + offset)?);
                    i += 1;
                    0
                }
                opcodes::mem::SETAT => {
                    let field_id = stack.pop().ok_or(Error::MissingValue(String::from("setat")))?.to_value() as usize;
                    let this_type = vm.mem.get_type(vm, this)?;
                    let offset = this_type.field_table
                        .get(self.version)
                        .ok_or(Error::VersionError(
                            format!("method version: {}; max type version: {}",
                                    self.version,
                                    this_type.field_table.len() - 1))
                        )?.clone();
                    let val = stack.pop().ok_or(Error::MissingValue(String::from("set")))?;
                    vm.mem.write_data(this, val, field_id + offset)?;
                    i += 1;
                    0
                }
                opcodes::mem::SIZE => {
                    let mem = vm.mem.mem.get(&this).ok_or(Error::UnknownPointer(this))?.2.clone();
                    match mem {
                        memory::Segment::Fields(data) => {
                            stack.push(Value(data.len() as u32))
                        }
                        memory::Segment::Bytes(data) => {
                            stack.push(Value(data.len() as u32))
                        }
                    }
                    i += 1;
                    0
                }
                opcodes::mem::EXPLODE => {
                    let ptr = stack.pop().ok_or(Error::MissingValue(String::from("explode")))?.to_value();
                    let memory::Segment::Fields(mem) = vm.mem.mem.get(&ptr).ok_or(Error::UnknownPointer(ptr))?.2.clone() else {
                        return Err(Error::BytesNotObject(String::from("explode")));
                    };
                    for field in mem {
                        stack.push(field);
                    }
                    i += 1;
                    0
                }
                opcodes::mem::APPEND => {
                    let val = stack.pop().ok_or(Error::MissingValue(String::from("append")))?;
                    let ptr = stack.pop().ok_or(Error::MissingValue(String::from("append")))?.to_value();
                    let mem = vm.mem.mem.get_mut(&ptr).ok_or(Error::UnknownPointer(ptr))?;
                    let mut fields = mem.2.clone();
                    match &mut fields {
                        memory::Segment::Fields(data) => {
                            data.push(val);
                        }
                        memory::Segment::Bytes(data) => {
                            data.extend(val.to_bytes())
                        }
                    }
                    mem.2 = fields;
                    i += 1;
                    0
                }
                opcodes::io::ECHO => {
                    let str_ptr = stack.pop().ok_or(Error::MissingValue(String::from("echo")))?.to_value();
                    let (_, type_name, bytes) = vm.mem.mem.get(&str_ptr).ok_or(Error::UnknownPointer(str_ptr))?;
                    if **type_name != "String" {
                        return Err(Error::TypeError(type_name.to_string(), String::from("String")));
                    }
                    let Segment::Bytes(bytes) = bytes else {
                        return Err(Error::ObjectNotBytes(format!("String being fed to `echo` should be a utf-8 byte array of type `String`")));
                    };
                    let string = String::try_from(bytes.clone()).map_err(|_| Error::IOError(String::from("Invalid string")))?;
                    print!("{string}");
                    std::io::stdout().flush().map_err(|e| Error::IOError(e.to_string()))?;
                    i += 1;
                    0
                }
                opcodes::io::INPUT => {
                    let mut str = String::new();
                    std::io::stdin().read_line(&mut str).map_err(|_| Error::IOError(String::from("Failed to read line")))?;
                    let str = str.replace("\n","");
                    let str = str.replace("\r","");
                    let str = str.as_bytes();
                    let str_type_name = String::from("String");
                    let ptr = vm.mem.alloc_blank(str_type_name);
                    vm.mem.write_all(ptr, memory::Segment::Bytes(str.to_vec()))?;
                    stack.push(Pointer(ptr));
                    i += 1;
                    0
                }
                opcodes::primitive::MINT => {
                    stack.push(Value(bytes_to_u32(&self.code[i + 1..i + 5])));
                    i += 5;
                    0
                }
                opcodes::primitive::LSTR => {
                    let type_ = vm.mem.get_type(vm, this)?;
                    let const_idx = bytes_to_u32(&self.code[i+1..i+5]) as usize;
                    let str = String::try_from(type_.constants[self.version][const_idx].to_vec()).map_err(|_| Error::ConstInterpretationError(String::from("str")))?;
                    let str: &[u8] = str.as_bytes();
                    let str_type_name = String::from("String");
                    let ptr = vm.mem.alloc_blank(str_type_name);
                    vm.mem.write_all(ptr, memory::Segment::Bytes(str.to_vec()))?;
                    stack.push(Pointer(ptr));
                    i += 5;
                    0
                }
                opcodes::primitive::MARR => {
                    let length = bytes_to_u32(&self.code[i+1..i+5]) as usize;
                    let mut values = Vec::new();
                    for _ in 0..length {
                        values.push(stack.pop().ok_or(Error::MissingValue(String::from("marr")))?);
                    }
                    values.reverse();
                    let ptr = vm.mem.alloc_blank(String::from("Array"));
                    vm.mem.write_all(ptr, memory::Segment::Fields(values))?;
                    stack.push(Data::Pointer(ptr));
                    i += 5;
                    0
                }
                opcodes::operation::SEND => {
                    let ptr: u32 = stack.pop().ok_or(Error::MissingValue(String::from("send")))?.to_value();
                    let this_type = vm.mem.get_type(vm, this)?;
                    let message: String = String::try_from(this_type
                        .constants[self.version]
                        .get(bytes_to_u32(&self.code[i + 1..i + 5]) as usize)
                        .ok_or(Error::ConstInterpretationError(
                            String::from("Const index provided in message send not found")
                        ))?
                        .to_vec())
                        .map_err(|_| Error::ConstInterpretationError(String::from("message_name in `send` operation")))?;
                    let that_type = vm.mem.get_type(vm, ptr)?;
                    let message = that_type.messages.get(&message).ok_or(Error::MessageNotFound(message))?.clone();
                    let mut args: Vec<Data> = Vec::new();
                    let arg_count = message.args;
                    for _ in 0..arg_count {
                        args.push(stack.pop().ok_or(Error::MissingValue(String::from("arg for `send` operation")))?);
                    }
                    args.reverse();
                    if let Some(ret_val) = message.call(ptr, vm, args)? {
                        stack.push(ret_val);
                    }
                    i += 5;
                    0
                }
                opcodes::operation::ADD_INT => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(((a as i32) + (b as i32)) as u32));
                    i += 1;
                    0
                }
                opcodes::operation::SUB_INT => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(((a as i32) - (b as i32)) as u32));
                    i += 1;
                    0
                }
                opcodes::operation::MUL_INT => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(((a as i32) * (b as i32)) as u32));
                    i += 1;
                    0
                }
                opcodes::operation::DIV_INT => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(((a as i32) / (b as i32)) as u32));
                    i += 1;
                    0
                }
                opcodes::operation::REM_INT => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(((a as i32) % (b as i32)) as u32));
                    i += 1;
                    0
                }
                opcodes::operation::LESS_INT => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(((a as i32) < (b as i32)) as u32));
                    i += 1;
                    0
                }
                opcodes::operation::MORE_INT => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(((a as i32) < (b as i32)) as u32));
                    i += 1;
                    0
                }
                opcodes::operation::EQUAL => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value((a == b) as u32));
                    i += 1;
                    0
                }
                opcodes::bitwise::SHR => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(a >> b));
                    i += 1;
                    0
                }
                opcodes::bitwise::SHL => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(a << b));
                    i += 1;
                    0
                }
                opcodes::bitwise::AND => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(a & b));
                    i += 1;
                    0
                }
                opcodes::bitwise::OR => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(a | b));
                    i += 1;
                    0
                }
                opcodes::bitwise::XOR => {
                    let b: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    let a: u32 = stack.pop().ok_or(Error::MissingValue(String::from("b")))?.to_value();
                    stack.push(Value(a ^ b));
                    i += 1;
                    0
                }
                opcodes::bitwise::NOT => {
                    let val: u32 = stack.pop().ok_or(Error::MissingValue(String::from("a")))?.to_value();
                    stack.push(Value(!val));
                    i += 1;
                    0
                }
                opcodes::var::LOCAL => {
                    let local_idx = bytes_to_u32(&self.code[i+1..i+5]) as usize;
                    if locals.len() <= local_idx {
                        locals.extend(vec![Value(0); local_idx - locals.len() + 1]);
                    }
                    locals[local_idx] = stack.pop().ok_or(Error::MissingValue(String::from("local")))?;
                    i += 5;
                    0
                }
                opcodes::var::LOAD => {
                    let local_idx = bytes_to_u32(&self.code[i+1..i+5]) as usize;
                    stack.push(*locals.get(local_idx).ok_or(Error::MissingLocal(local_idx))?);
                    i += 5;
                    0
                }
                _ => return Err(Error::NotImplemented(format!("Instruction 0x{:X}", instruction))),
            };
            if vm.debug > 0 {
                println!("\t\tStack: {stack:?}");
            }
            if vm.debug > 1 {
                println!("\t\tMemory:\n\t\t{}", format!("{:#?}", vm.mem).replace("\n","\n\t\t"));
            }
        };
        for val in stack {
            if let Data::Pointer(ptr) = val {
                vm.mem.free(ptr)?;
            }
        }
        for val in locals {
            if let Data::Pointer(ptr) = val {
                vm.mem.free(ptr)?;
            }
        }
        if vm.debug > 0 {
            println!("Method exiting (returned void)");
        }
        Ok(None)
    }
}