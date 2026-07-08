use std::collections::HashMap;
use std::rc::Rc;
use crate::parse::Node;

macro_rules! hashmap {
    ($($key:literal : $value:expr),+ $(,)?) => {{
        let mut hashmap = std::collections::HashMap::new();
        $(
            hashmap.insert($key, $value);
        )+
        hashmap
    }};
    ($($key:literal => $value:expr),+ $(,)?) => {{
        let mut hashmap = std::collections::HashMap::new();
        $(
            hashmap.insert($key, $value);
        )+
        hashmap
    }};
    ($($key:expr => $value:expr),+ $(,)?) => {{
        let mut hashmap = std::collections::HashMap::new();
        $(
            hashmap.insert($key, $value);
        )+
        hashmap
    }};
}

pub static VM_DIR: include_dir::Dir = include_dir::include_dir!("$CARGO_MANIFEST_DIR/vm");

#[derive(Debug, Clone)]
pub enum Error {
    SyntaxError(String),
    IOError(String),
    NotImplemented(String),
    ImpossibleError(String),
    Exception(vm::Error),
    CommandLineError(String),
}
type Result<T> = std::result::Result<T, Error>;

mod parse;
mod build;
mod compile;
mod vm;

fn make_file(path: String) -> Result<Node> {
    let file = std::fs::read_to_string(path.clone()).map_err(
        |err| Error::IOError(err.to_string() + format!(" ({})", path).as_str())
    )?;
    parse::parse(file)
}

pub fn make(paths: Vec<String>, from_dir: &str, dir: &str) -> std::result::Result<Vec<String>, Error> {
    let mut nodes: Vec<Node> = Vec::new();
    for path in paths.clone() {
        nodes.push(make_file(from_dir.to_string() + path.as_str())?);
    }
    let types = build::build(nodes)?;
    let mut files: Vec<(String, Rc<[u8]>)> = Vec::new();
    for type_ in types {
        files.push((type_.name.clone(), compile::compile(type_)?));
    }
    let mut type_mods: HashMap<String, usize> = HashMap::new();
    let mut written_paths: Vec<String> = Vec::new();
    for ((_, file), name) in std::iter::zip(files, paths) {
        let is_mod: bool = file[7] & 0b00000001 != 0;
        if is_mod {
            type_mods.insert(
                name.clone(),
                type_mods.get(&name)
                    .map_or(
                        0, |v| v + 1
                    )
            );
        }
        let path = (name
            .replace("\\", "/")
            .strip_suffix(".oovmm")
            .unwrap_or(name.as_str())
            .strip_suffix(".oovmt")
            .unwrap_or(name
                .strip_suffix(".oovmm")
                .unwrap_or(name.as_str()))
            .to_string()
        + ".")
        .replace("<", "oovm.")
        .replace(">", ".magic")
        .replace("..", ".")
        + (
            if !is_mod { String::from("type") }
            else { (type_mods.get(&name).unwrap()).to_string() + ".mod" }
        ).as_str();
        let path = path.replace("\\","/");
        let path = *path.split('/').collect::<Vec<&str>>().last().unwrap();
        let path = dir.to_string() + path;
        written_paths.push(path.clone());
        std::fs::write(path, file)
            .map_err(
                |err| Error::IOError(err.to_string())
            )?;
    };
    Ok(written_paths)
}

pub use vm::Vm;

pub fn exec(paths: Vec<String>, debug: u8) -> std::result::Result<u32, Error> {
    vm::main(paths, debug).map_err(Error::Exception)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect::<Vec<_>>();
    if if args.len() > 1 {args[1] == "make"} else {false} {
        let paths: Vec<String> = args[2..].to_vec();
        make(paths, ".", ".")?;
    }
    if if args.len() > 1 {args[1] == "make-into"} else {false} {
        let paths: Vec<String> = args[3..].to_vec();
        make(paths, "", args[2].as_str())?;
    }
    else if if args.len() > 1 {args[1] == "run"} else {false} {
        exec(args[2..].to_vec(), 0)?;
    }
    else if if args.len() > 1 {args[1] == "debug"} else {false} {
        if args.len() < 3 {
            println!("Invalid arguments for command `debug`; check usage with `oovm --help` to see correct values.");
        }
        exec(args[3..].to_vec(), u8::from_str_radix(args[2].as_str(), 10).map_err(|_| Error::CommandLineError(format!("Cannot parse debug level from argument `{}`", args[2])))?)?;
    }
    else {
        println!("Usage:
    oovm make [paths]+
        : Takes files from the oovm text format into oovm bytecode to be executed
    oovm run [paths]+
        : Runs files in oovm bytecode format
    oovm debug [level] [paths]+
        : Runs files in oovm bytecode format in debug mode at the provided debug level");
    }
    Ok(())
}
