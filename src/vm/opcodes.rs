pub mod operation_2 {
    /// # **`arr1:ptr, arr2:ptr` → `concat` ⇒ `arr3:ptr`**
    /// Concatenates the contents of `arr1` with the contents of `arr2` into a new array titles `arr3`
    /// ```
    /// mint 12
    /// mint 13
    /// mint 14
    /// marr 3
    /// mint 2
    /// mint 3
    /// mint 4
    /// marr 3
    /// concat
    /// ```
    /// Creates two arrays, one with the contents `[12,13,14]`, and the other with the contents `[2,3,4]`. The two arrays are then concatenated with a resulting array of `[12,13,14,2,3,4]`
    /// (0x70:112)
    pub const CONCAT: u8 = 0x70;
    pub const MEM_EQUAL: u8 = 0x71;
    pub const SEND_DYNAMIC: u8 = 0x72;
}
pub mod control {
    pub const RET: u8 = 0x80;
    pub const UNLESS: u8 = 0x81;
    pub const GOTO: u8 = 0x82;
    pub const EXIT: u8 = 0x83;
}
pub mod stack {
    pub const DUP: u8 = 0x90;
    pub const DUPN: u8 = 0x91;
    pub const SWAP: u8 = 0x92;
    pub const REV: u8 = 0x93;
}
pub mod mem {
    /// # **`new type:constp_idx` ⇒ `ptr`**
    ///
    /// Creates a new allocation of the type provided with its size predetermined by type.
    /// ```
    /// new 1
    /// send 2
    /// ```
    /// const_pool:
    /// ```
    /// String "ExampleType"
    /// String "example_message"
    /// ```
    /// Creates an instance of the `ExampleType` type and send `ExampleMessage` to it.
    /// (0xA0:160)
    pub const NEW: u8 = 0xA0;
    pub const FREE: u8 = 0xA2;
    pub const REF: u8 = 0xA3;
    pub const SET: u8 = 0xA4;
    pub const GET: u8 = 0xA5;
    /// # **`main` ⇒ `main_ptr`**
    ///
    /// Returns a pointer to the global instance of type `<>`.
    /// ```
    /// main
    /// send 0
    /// ```
    /// const_pool:
    /// ```
    /// "static_method"
    /// ```
    /// Calls the static method `static_method` defined in `oovm.magic.n.mod` with any value of `n`.
    pub const MAIN: u8 = 0xA6;
    pub const THIS: u8 = 0xA7;
    pub const GETAT: u8 = 0xA8;
    pub const SETAT: u8 = 0xA9;
    pub const SIZE: u8 = 0xAA;
    pub const EXPLODE: u8 = 0xAB;
    pub const APPEND: u8 = 0xAC;
    pub const TYPEOF: u8 = 0xAD;
}
/// In/Out Instructions such as **`echo`** and **`input`**, all of the form *`0xB`***`N`** where **`N`** is the specific instruction number
pub mod io {
    /// # **`str:ptr` → `echo`**
    ///
    /// Prints the contents of a string pointer.
    /// If the string pointer points to any other type,
    /// the program will crash with a Exception::TypeError(typeof($str)).
    /// ```
    /// lstr 0
    /// echo
    /// ```
    /// Loads the typename of the class the method belongs to and prints it.
    /// (0xB0:176)
    pub const ECHO: u8 = 0xB0;
    pub const INPUT: u8 = 0xB1;
    pub const READ_FILE: u8 = 0xB2;
    /// # **`str:ptr`, `bytearr` → `wfile`**
    ///
    /// Writes the bytes in the byte string `$bytearr` to the file with the path in `$str` at index `$idx`.
    /// If a String is passed as `$bytearr`, it will write the exact bytes of the string to the file.
    /// If instead a character array (or a regular integer array) is passed as `$bytearr`, the values will be written
    /// in utf-32, not compressed into utf-8.
    /// ```
    /// lstr "file.txt"
    /// lstr "Hello, World!"
    /// wfile
    /// ```
    /// Writes "`Hello, World!`" to `file.txt`.
    pub const WRITE_FILE: u8 = 0xB3;
    pub const DELETE_FILE: u8 = 0xB4;
}
pub mod primitive {
    pub const MINT: u8 = 0xC0;
    pub const MSTR: u8 = 0xC1;
    pub const LSTR: u8 = 0xC3;
    pub const MARR: u8 = 0xC4;
    pub const CHARS: u8 = 0xC5;
}
pub mod operation {
    pub const SEND: u8 = 0xD0;
    pub const ADD_INT: u8 = 0xD1;
    pub const SUB_INT: u8 = 0xD2;
    pub const MUL_INT: u8 = 0xD3;
    pub const DIV_INT: u8 = 0xD4;
    pub const REM_INT: u8 = 0xD5;
    pub const ADD_FLOAT: u8 = 0xD6;
    pub const SUB_FLOAT: u8 = 0xD7;
    pub const MUL_FLOAT: u8 = 0xD8;
    pub const DIV_FLOAT: u8 = 0xD9;
    pub const REM_FLOAT: u8 = 0xDA;
    pub const LESS_INT: u8 = 0xDB;
    pub const LESS_FLOAT: u8 = 0xDC;
    pub const MORE_INT: u8 = 0xDD;
    pub const MORE_FLOAT: u8 = 0xDE;
    pub const EQUAL: u8 = 0xDF;
}
pub mod bitwise {
    pub const SHR: u8 = 0xE0;
    pub const SHL: u8 = 0xE1;
    pub const AND: u8 = 0xE2;
    pub const OR: u8 = 0xE3;
    pub const XOR: u8 = 0xE4;
    pub const NOT: u8 = 0xE5;
    pub const NOT_BOOL: u8 = 0xE6;
}
pub mod var {
    pub const LOCAL: u8 = 0xF0;
    pub const LOAD: u8 = 0xF1;
}