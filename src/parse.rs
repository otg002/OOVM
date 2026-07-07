use crate::{Error, Result};
use std::cell::RefCell;
use std::fmt::Display;
use std::rc::Rc;

fn tokenize(program: String) -> Result<Vec<String>> {
    let mut tokens: Vec<String> = Vec::new();
    let mut word: String = String::new();
    let mut string: bool = false;
    for c in program.chars() {
        if string {
            if c == '"' {
                string = false;
                word.push('"');
                tokens.push(word);
                word = String::new();
            } else {
                word.push(c);
            }
        } else {
            if c.is_whitespace() {
                if !word.is_empty() {
                    tokens.push(word);
                    word = String::new();
                }
            } else if c == '"' {
                if !word.is_empty() {
                    tokens.push(word);
                }
                string = true;
                word = String::from('"');
            } else if (c == '{') || (c == '{') || (c == '[') || (c == '(') || (c == ')') || (c == ']') || (c == '}') {
                if !word.is_empty() {
                    tokens.push(word);
                }
                tokens.push(String::from(c));
                word = String::new();
            } else {
                word.push(c);
            }
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    if tokens.is_empty() {
        Err(Error::SyntaxError(String::from("OOVM text file found empty")))
    }
    else if !(&tokens[0] == "type" || &tokens[0] == "mod") {
        Err(Error::SyntaxError(String::from("OOVM text files must begin with `type` or `mod`")))
    }
    else {
        Ok(tokens)
    }
}

#[derive(Clone)]
struct CyclicTree<T: Clone> {
    data: T,
    parent: Option<Rc<RefCell<CyclicTree<T>>>>,
    children: Vec<Rc<RefCell<CyclicTree<T>>>>,
}
impl<T: Clone> CyclicTree<T> {
    fn new(parent: Option<Tree<T>>, data: T) -> Tree<T> {
        Rc::new(RefCell::new(Self {
            parent,
            data,
            children: Vec::new(),
        }))
    }
    fn add(&mut self, child: Tree<T>) -> Tree<T> {
        self.children.push(child.clone());
        child
    }
}
type Tree<T> = Rc<RefCell<CyclicTree<T>>>;
#[derive(Clone, Debug)]
pub struct FlatTree<T> {
    data: T,
    children: Rc<[FlatTree<T>]>,
}
impl<T> FlatTree<T> {
    fn new(data: T, children: Vec<FlatTree<T>>) -> FlatTree<T> {
        Self {
            data,
            children: children.into(),
        }
    }
}
impl<T: Display> Display for FlatTree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.data, {
            if self.children.is_empty() {
                String::from("")
            }
            else {
                let mut kid_str: String = String::from(" {");
                for child in self.children.iter() {
                    kid_str.push_str(format!("\n\t{}", child.to_string().replace("\n", "\n\t")).as_str())
                }
                kid_str.push_str("\n}");
                kid_str
            }
        })
    }
}

impl<T: Clone> From<CyclicTree<T>> for FlatTree<T> {
    fn from(cyclic: CyclicTree<T>) -> Self {
        let mut kids: Vec<FlatTree<T>> = Vec::with_capacity(cyclic.children.len());
        for child in cyclic.children {
            kids.push(child.borrow().clone().into());
        }
        FlatTree::new(cyclic.data, kids)
    }
}
#[derive(Clone, Debug, PartialEq)]
enum Symbol {
    Header(bool),
    Root,
    Ident(String),
    Method,
    Block,
    Object,
    Bytes,
}
impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Symbol::Ident(ident) => write!(f, "'{}'", ident),
            _ => write!(f, "{:?}", self),
        }
    }
}

fn to_ast(program: String) -> Result<FlatTree<Symbol>> {
    let tokens = tokenize(program)?;
    let mut current: Tree<Symbol> = CyclicTree::new(None, Symbol::Root);
    for token in tokens {
        match token.as_str() {
            "{" => {
                let next = CyclicTree::new(Some(current.clone()), Symbol::Block);
                let next = current.borrow_mut().add(next);
                current = next;
            }
            "}" => {
                let Symbol::Block = current.borrow().data else {
                    return Err(Error::SyntaxError(String::from("Unexpected `}` symbol - this may be due to misplaced `]` or `)` symbols or due to an unmatched `}`")));
                };
                let parent = current.borrow().parent.clone().ok_or(Error::SyntaxError(String::from("Unmatched `}`")))?;
                current = parent;
                let kind = current.borrow().data.clone();
                match kind {
                     Symbol::Method => {
                        let parent = current.borrow().parent.clone().ok_or(Error::SyntaxError(String::from("Unmatched `}`"))) ?;
                        current = parent;
                    }
                    _ => ()
                }
            }
            "type" => {
                let next = CyclicTree::new(Some(current.clone()), Symbol::Header(false));
                let next = current.borrow_mut().add(next);
                current = next;
            }
            "mod" => {
                let next = CyclicTree::new(Some(current.clone()), Symbol::Header(true));
                let next = current.borrow_mut().add(next);
                current = next;
            }
            "method" => {
                let next = CyclicTree::new(Some(current.clone()), Symbol::Method);
                let next = current.borrow_mut().add(next);
                current = next;
            }
            "object" => {
                let ident = CyclicTree::new(Some(current.clone()), Symbol::Object);
                current.borrow_mut().add(ident);
            }
            "bytes" => {
                let ident = CyclicTree::new(Some(current.clone()), Symbol::Bytes);
                current.borrow_mut().add(ident);
            }
            "#" => {
                let parent = current.borrow().parent.clone().ok_or(Error::SyntaxError(String::from("Unexpected `#`")))?;
                current = parent;
            }
            _ => {
                let ident = CyclicTree::new(Some(current.clone()), Symbol::Ident(token));
                current.borrow_mut().add(ident);
            }
        }
    }
    Ok(current.borrow().clone().into())
}

type TypeName = String;

#[derive(Clone, Debug, PartialEq)]
pub enum Node {
    Header {
        object: bool,
        mix_in: bool,
        name: TypeName,
        size: usize,
    },
    Root(Vec<Node>),
    Method {
        name: String,
        args: u32,
        code: Vec<String>,
    },
}

fn compose(ast: &FlatTree<Symbol>) -> Result<Node> {
    match ast.data {
        Symbol::Root => {
            let mut nodes: Vec<Node> = Vec::with_capacity(ast.children.len());
            for child in ast.children.iter() {
                nodes.push(compose(child)?);
            }
            Ok(Node::Root(nodes))
        }
        Symbol::Block => Err(Error::SyntaxError(String::from("Unexpected `{...}` pair outside of method or field descriptor"))),
        Symbol::Ident(_) => Err(Error::SyntaxError(String::from("Unexpected identifier outside method or field descriptor"))),
        Symbol::Method => {
            if ast.children.len() != 3 {
                Err(Error::SyntaxError(String::from("Malformed method definition")))
            }
            else {
                if let Symbol::Ident(method_name) = ast.children[0].data.clone() {
                    if let Symbol::Ident(method_arg_count) = ast.children[1].data.clone() {
                        let method_arg_count = method_arg_count.parse::<u32>().map_err(|_| Error::SyntaxError(String::from("Invalid method argument count - could not parse as integer")))?;
                        if let FlatTree {data: Symbol::Block, children} = ast.children[2].clone() {
                            let mut code: Vec<String> = Vec::with_capacity(children.len());
                            for tk in children.iter() {
                                let FlatTree {data: Symbol::Ident(tk), children: _} = tk.clone() else {
                                    return Err(Error::SyntaxError(String::from("Methods may only contain identifiers in body, no other symbols are allowed")));
                                };
                                code.push(tk);
                            }
                            Ok(Node::Method {name: method_name, args: method_arg_count, code: code})
                        }
                        else {
                            Err(Error::SyntaxError(String::from("Method missing implementation")))
                        }
                    }
                    else {
                        Err(Error::SyntaxError(String::from("Method missing argument count")))
                    }
                }
                else {
                    Err(Error::SyntaxError(String::from("Method missing name")))
                }
            }
        }
        Symbol::Header(mix_in) => {
            if ast.children.len() != 3 {
                return Err(Error::SyntaxError(String::from("Malformed header (headers should consist of 3 elements)")));
            }
            if ast.children[0].data != Symbol::Object && ast.children[0].data != Symbol::Bytes {
                return Err(Error::SyntaxError(String::from("Malformed header (missing either `object` or `bytes`)")));
            }
            let object: bool = ast.children[0].data == Symbol::Object;
            if let Symbol::Ident(class_name) = ast.children[1].data.clone() {
                let Symbol::Ident(num) = ast.children[2].data.clone() else {
                    return Err(Error::SyntaxError(String::from("Malformed header (missing size)")));
                };
                let size = usize::from_str_radix(&num.to_string(), 10).map_err(|_| Error::SyntaxError(String::from("Malformed header (size could not be parsed)")))?;
                Ok(Node::Header { object, mix_in, name: class_name, size })
            }
            else {
                Err(Error::SyntaxError(String::from("Header missing class name")))
            }
        }
        Symbol::Object => {
            return Err(Error::SyntaxError(String::from("Keyword `object` may only be found after `type` or `mod`.")));
        }
        Symbol::Bytes => {
            return Err(Error::SyntaxError(String::from("Keyword `bytes` may only be found after `type` or `mod`.")));
        }
    }
}

pub fn parse(program: String) -> Result<Node> {
    let ast = to_ast(program)?;
    compose(&ast)
}