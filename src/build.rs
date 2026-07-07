use std::collections::HashMap;
use crate::{Error, Result};
use crate::parse::Node;

type TypeName = String;
#[derive(Debug, Clone)]
pub struct Type {
    pub object: bool,
    pub mix_in: bool,
    pub name: TypeName,
    pub size: u32,
    pub messages: HashMap<String, Method>,
}
#[derive(Debug, Clone)]
pub struct Method {
    pub args: u32,
    pub code: Vec<String>,
}

fn node_to_type(node: Node) -> Result<Type> {
    let Node::Root(elements) = node else {
        return Err(Error::ImpossibleError(format!("Root missing when converting node to type: {:?}", node)))
    };
    let Node::Header {object, mix_in, mut name, size} = elements[0].clone() else {
        return Err(Error::SyntaxError(String::from("Nodes must begin with `type <TypeName> ([fields])` header")))
    };
    if &name[0..1] != "\"" || &name[name.len() - 1..name.len()] != "\"" {
        return Err(Error::SyntaxError(String::from("Type name must be quoted")))
    }
    name.remove(0);
    name.pop();
    let mut messages: HashMap<String, Method> = HashMap::new();
    for method in elements[1..].iter() {
        let Node::Method { name: mut method_name, args, code } = method.clone() else {
            return Err(Error::SyntaxError(format!("Malformed type file - expected method")))
        };
        if &method_name[0..1] != "\"" || &method_name[method_name.len() - 1..method_name.len()] != "\"" {
            return Err(Error::SyntaxError(format!("Malformed method name - expected name of method to be quoted")));
        }
        method_name.remove(0);
        method_name.pop();
        messages.insert(method_name, Method { args, code });
    }
    let size = size as u32;
    Ok(Type {
        object,
        mix_in,
        name,
        size,
        messages,
    })
}
pub fn build(nodes: Vec<Node>) -> Result<Vec<Type>> {
    let mut types: Vec<Type> = Vec::new();
    for node in nodes {
        let type_ = node_to_type(node)?;
        types.push(type_);
    }
    Ok(types)
}