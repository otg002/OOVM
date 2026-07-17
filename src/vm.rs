use std::collections::HashMap;
use std::rc::Rc;
use crate::VM_DIR;

#[derive(Debug, Clone)]
pub enum Error {
    IOError(String),
    NotImplemented(String),
    ImpossibleError(String),
    TypeLoadError(String),
    TypeNotFound(String),
    NameError(String),
    UnknownPointer(u32),
    MissingValue(String),
    ConstInterpretationError(String),
    MessageNotFound(TypeName, String),
    MissingLocal(usize),
    /// (found, expected)
    TypeError(String, String),
    /// (length, index)
    IndexOutOfBounds(usize, usize),
    VersionError(String),
    Panic,
    BytesNotObject(String),
    ObjectNotBytes(String),
    ExpectedPointer,
}
type Result<T> = std::result::Result<T, Error>;

type TypeName = String;

#[derive(Debug, Clone)]
struct Type {
    object: bool,
    size: usize,
    messages: HashMap<String, Rc<Method>>,
    constants: Vec<Rc<[Rc<[u8]>]>>,
    field_table: Vec<usize>,
}

mod memory;
use memory::*;

pub mod opcodes;

#[derive(Clone, Debug, Copy, PartialEq)]
enum Data {
    Value(u32),
    Pointer(u32),
}
impl Data {
    fn to_bytes(&self) -> Vec<u8> {
        match self {
            Value(n) => n.to_be_bytes().to_vec(),
            Pointer(n) => n.to_be_bytes().to_vec(),
        }
    }
    fn to_value(&self) -> u32 {
        match self {
            Value(n) => *n,
            Pointer(n) => *n,
        }
    }
}
use Data::*;

struct MethodVersionless {
    args: u32,
    code: Rc<[u8]>,
}
impl MethodVersionless {
    fn to_method(&self, version: usize) -> Method {
        Method::new(
            version,
            self.args,
            self.code.clone(),
        )
    }
}

mod method;
use method::*;

#[derive(Debug)]
pub struct Vm {
    types: HashMap<String,Type>,
    mem: Memory,
    debug: u8,
}

fn bytes_to_u32(bytes: &[u8]) -> u32 {
    u32::from_be_bytes(<[u8; 4]>::try_from(bytes).unwrap())
}
fn load_type(vm: &mut Vm, file: Vec<u8>) -> Result<()> {
    if &file[0..4] != &['O' as u8, 'O' as u8, 'V' as u8, 'M' as u8] {
        return Err(Error::TypeLoadError(String::from("Type file missing correct magic numbers")));
    }
    let flags = file[7];
    let mixin: bool = (flags & 0b00000001) != 0;
    let object: bool = (flags & 0b00000010) != 0;
    let type_pool_index: u32 = bytes_to_u32(&file[8..12]);
    let size: u32 = bytes_to_u32(&file[12..16]);
    let mut byte: usize = 16;
    let message_count: u32 = bytes_to_u32(&file[byte..byte+4]);
    byte += 4;
    let mut messages: Vec<(u32, MethodVersionless)> = Vec::new();
    let mut loaded_messages: u32 = 0;
    while loaded_messages < message_count {
        let name_index = bytes_to_u32(&file[byte..byte+4]);
        byte += 4;
        let args = bytes_to_u32(&file[byte..byte+4]);
        byte += 4;
        let mut code: Vec<u8> = Vec::new();
        let size = bytes_to_u32(&file[byte..byte+4]);
        byte += 4;
        for _ in 0..size {
            code.push(file[byte]);
            byte += 1;
        }
        messages.push((name_index, MethodVersionless {
            args,
            code: code.into(),
        }));
        loaded_messages += 1;
    }
    let mut const_pool: Vec<Vec<u8>> = Vec::new();
    let const_count = bytes_to_u32(&file[byte..byte+4]);
    let mut consts: u32 = 0;
    byte += 4;
    while consts < const_count {
        if byte + 4 > file.len() {
            return Err(
                Error::TypeLoadError(
                    format!("Ran out of bytes while loading length of constant {consts} at byte {byte} for file length {}", file.len())));
        }
        let len = bytes_to_u32(&file[byte..byte+4]) as usize;
        byte += 4;
        if byte + len > file.len() {
            return Err(
                Error::TypeLoadError(
                    format!("Ran out of bytes while loading constant {consts} at byte {byte} for file length {}", file.len())));
        }
        let bytes = Vec::from(&file[byte..byte + len]);
        const_pool.push(bytes);
        byte += len;
        consts += 1;
    }
    let name: String = String::try_from(
        const_pool[type_pool_index as usize]
            .clone())
        .map_err(|err| Error::TypeLoadError(format!("While trying to parse type name: {err}")))?;
    let mut imut_const_pool: Vec<Rc<[u8]>> = Vec::new();
    let version = if mixin {
        let mod_type: &Type = vm.types.get(&name)
            .ok_or(
                Error::TypeNotFound(
                    format!("Type `{name}` not found while attempting to modify")))?;
        mod_type.constants.len()
    } else {0};
    for const_ in const_pool {
        imut_const_pool.push(const_.into());
    }
    let mut methods: HashMap<String, Rc<Method>> = HashMap::new();
    for (name_idx, method) in messages {
        let name = String::try_from(imut_const_pool[name_idx as usize].to_vec())
            .map_err(|err| Error::TypeLoadError(format!("While trying to parse message: {err}")))?;
        methods.insert(name, Rc::new(method.to_method(version)));
    }
    if mixin {
        let mod_type: &mut Type = vm.types
            .get_mut(&name).ok_or(
            Error::TypeNotFound(format!("Type `{name}` not found while attempting to modify")))?;
        for (name, message) in methods {
            mod_type.messages.insert(name, message);
        }
        let consts: &mut Vec<_> = &mut mod_type.constants;
        consts.push(imut_const_pool.into());
        mod_type.size += size as usize;
        mod_type.field_table.push(mod_type.size);
    }
    else {
        let loaded: Type = Type {
            size: size as usize,
            object,
            messages: methods,
            constants: vec![imut_const_pool.into()],
            field_table: vec![0],
        };
        vm.types.insert(name, loaded);
    }
    Ok(())
}
fn load_from_file(vm: &mut Vm, path: String) -> Result<()> {
    load_type(vm, std::fs::read(path).map_err(|err| Error::IOError(err.to_string()))?.into())
}

pub fn main(paths: Vec<String>, debug: u8) -> Result<u32> {
    let mut vm = Vm {
        types: HashMap::new(),
        mem: Memory::new(),
        debug,
    };
    load_type(&mut vm, VM_DIR.get_file("builtin-types/oovm.magic.type").unwrap().contents().to_vec())?;
    load_type(&mut vm, VM_DIR.get_file("builtin-types/String.type").unwrap().contents().to_vec())?;
    load_type(&mut vm, VM_DIR.get_file("builtin-types/Array.type").unwrap().contents().to_vec())?;
    for path in paths {
        load_from_file(&mut vm, path)?;
    }
    if !vm.types.contains_key(&String::from("<>")) {
        Err(Error::TypeNotFound(String::from("<> (the entry point for the program should be <>::main)")))
    }
    else {
        vm.mem.alloc(String::from("<>"), vm.types.get(&String::from("<>")).unwrap().clone());
        //println!("types: {:#?}", vm.types);
        if debug > 0 {
            println!("Starting...");
        }
        let result = method::exec(&mut vm)?;
        if debug > 0 {
            println!("Memory: {:#?}", vm.mem);
        }
        Ok(result)
    }
}