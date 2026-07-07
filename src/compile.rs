use std::collections::HashSet;
use std::collections::HashMap;
use std::rc::Rc;
use std::str::FromStr as _;
use crate::{Error, Result};
use crate::build::{Type, Method};

type TypeName = String;

fn get_opcodes() -> HashMap<String, u8> {
    let opcodes: HashMap<&str, u8> = hashmap!{
        "concat": 0x70,
        "mem-equal": 0x71,
        "send.dyn": 0x72,

        "ret": 0x80,
        "unless": 0x81,
        "goto": 0x82,
        "exit": 0x83,

        "dup": 0x90,
        "dupn": 0x91,
        "swap": 0x92,
        "rev": 0x93,

        "new": 0xA0,
        "dealloc": 0xA1,
        "free": 0xA2,
        "ref": 0xA3,
        "set": 0xA4,
        "get": 0xA5,
        "main": 0xA6,
        "this": 0xA7,
        "getat": 0xA8,
        "setat": 0xA9,
        "size": 0xAA,
        "explode": 0xAB,

        "echo": 0xB0,
        "input": 0xB1,

        "mint": 0xC0,
        "mstr": 0xC1,
        "mfloat": 0xC2,
        "lstr": 0xC3,
        "marr": 0xC4,

        "send": 0xD0,
        "add.int": 0xD1,
        "sub.int": 0xD2,
        "mul.int": 0xD3,
        "div.int": 0xD4,
        "rem.int": 0xD5,
        "add.float": 0xD6,
        "sub.float": 0xD7,
        "mul.float": 0xD8,
        "div.float": 0xD9,
        "rem.float": 0xDA,
        "less.int": 0xDB,
        "less.float": 0xDC,
        "more.int": 0xDD,
        "more.float": 0xDE,
        "equal": 0xDF,

        "shr": 0xE0,
        "shl": 0xE1,
        "and": 0xE2,
        "or": 0xE3,
        "xor": 0xE4,
        "not": 0xE5,

        "local": 0xF0,
        "load": 0xF1,
    };
    let mut out = HashMap::new();
    for (opcode, value) in opcodes {
        out.insert(String::from(opcode), value);
    }
    out
}

fn delabel(program: Vec<String>) -> Result<Vec<String>> {
    let mut location: usize = 0;
    let mut default: Vec<String> = Vec::new();
    let mut numbers: HashSet<char> = HashSet::new(); {
        numbers.insert('0');
        numbers.insert('1');
        numbers.insert('2');
        numbers.insert('3');
        numbers.insert('4');
        numbers.insert('5');
        numbers.insert('6');
        numbers.insert('7');
        numbers.insert('8');
        numbers.insert('9');
    }
    for token in program.clone() {
        if token.chars().last().unwrap() == ':' {
            let original = token.clone();
            let mut name = String::from("#");
            name.push_str(token.as_str());
            name.pop();
            let mut output: Vec<String> = Vec::new();
            for token in program {
                if token == name {
                    output.push(format!("{}", location));
                }
                else if token != original {
                    output.push(token);
                }
            }
            return delabel(output);
        }
        else if token.chars().next() == Some('#') {
            default.push(token);
            location += 4;
        }
        else if numbers.contains(&token.chars().next().unwrap()) {
            default.push(token);
            location += 4;
        }
        else if token.chars().next().unwrap() == '"' {
            default.push(token);
            location += 4;
        }
        else {
            default.push(token);
            location += 1;
        }
    }
    Ok(default)
}
pub fn compile_code(program: Vec<String>, const_count: &mut usize, const_pool: &mut Vec<u8>) -> Result<Rc<[u8]>> {
    let mut output: Vec<u8> = Vec::new();
    let opcodes: HashMap<String, u8> = get_opcodes();
    let mut numbers: HashSet<char> = HashSet::new(); {
        numbers.insert('0');
        numbers.insert('1');
        numbers.insert('2');
        numbers.insert('3');
        numbers.insert('4');
        numbers.insert('5');
        numbers.insert('6');
        numbers.insert('7');
        numbers.insert('8');
        numbers.insert('9');
    }
    let delabeled = delabel(program)?;
    for token in delabeled {
        if opcodes.contains_key(&token) {
            output.push(opcodes.get(&token).unwrap().clone());
        } else if numbers.contains(&token.chars().next().unwrap()) {
            if token.contains(".") {
                output.extend_from_slice(&f32::from_str(&*token).map_err(|_| Error::SyntaxError(format!("Invalid number literal: {}", token)))?.to_be_bytes());
            }
            else {
                output.extend_from_slice(&u32::from_str_radix(&*token, 10).map_err(|_| Error::SyntaxError(format!("Invalid number literal: {}", token)))?.to_be_bytes());
            }
        } else if token.chars().next().unwrap() == '"' {
            let mut processed = String::new();
            let mut i: usize = 0;
            while i < token.len() {
                if &token[i..=i] == "\\" {
                    let hex_code = &token[i+1..i+3];
                    processed.push(u8::from_str_radix(hex_code, 16).map_err(|err| Error::SyntaxError(err.to_string()))? as char);
                    i += 3;
                }
                else {
                    processed.push_str(&token[i..=i]);
                    i += 1;
                }
            }
            let bytes: Vec<u8> = processed[1..processed.len()-1].bytes().collect();
            output.extend_from_slice(&(*const_count as u32).to_be_bytes());
            *const_count += 1;
            const_pool.extend_from_slice(&(processed.len() as u32 - 2).to_be_bytes());
            const_pool.extend_from_slice(bytes.as_slice());
        } else {
            return Err(Error::SyntaxError(format!("Invalid token: {}", token)));
        }
    }
    Ok(output.into())
}

pub fn compile(class: Type) -> Result<Rc<[u8]>> {
    let name: TypeName = class.name;
    let messages: HashMap<String, Method> = class.messages;
    let mix_in: bool = class.mix_in;
    let mut const_pool: Vec<u8> = Vec::new();
    let mut const_count: usize = 0;
    let mut const_map: HashMap<TypeName, usize> = HashMap::new();
    for byte in (name.len() as u32).to_be_bytes() {
        const_pool.push(byte);
    }
    for byte in name.clone().into_bytes() {
        const_pool.push(byte);
    }
    const_count += 1;
    const_map.insert(name, 0);
    let mut header: Vec<u8> = Vec::new();
    header.extend_from_slice(&[0,0,0,0]);
    header.extend(class.size.to_be_bytes());
    let mut message_body: Vec<u8> = Vec::new();
    message_body.extend_from_slice(&(messages.len() as u32).to_be_bytes());
    for (message_name, Method {args, code}) in messages {
        let const_index = const_map.get(&message_name).unwrap_or(&const_count).clone();
        if !const_map.contains_key(&message_name) {
            const_pool.extend_from_slice(&(message_name.len() as u32).to_be_bytes());
            const_pool.extend_from_slice(message_name.as_bytes());
            const_count += 1;
        }
        let code = &compile_code(code, &mut const_count, &mut const_pool)?.to_owned();
        message_body.extend_from_slice(&(const_index as u32).to_be_bytes());
        message_body.extend_from_slice(&args.to_be_bytes());
        message_body.extend_from_slice(&(code.len() as u32).to_be_bytes());
        message_body.extend_from_slice(code);
    }
    let mut output: Vec<u8> = Vec::new();
    output.extend_from_slice(&['O' as u8, 'O' as u8, 'V' as u8, 'M' as u8]);
    output.extend_from_slice(&[0,0,0,mix_in as u8 | ((class.object as u8) << 1)]);
    output.extend_from_slice(&header);
    output.extend_from_slice(&message_body);
    output.extend_from_slice(&(const_count as u32).to_be_bytes());
    output.extend_from_slice(&const_pool);
    Ok(output.into())
}