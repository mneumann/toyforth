use std::io::{self, BufRead};
use std::str::FromStr;

#[derive(Copy, Clone, Debug)]
enum CompiledInstruction {
    DUP,
    DROP,
    SWAP,
    ADD,
    SUB,
    MUL,
    DIV,
    CALL,
    JUMP,
    RET,
    NOP,

    // Used to push data onto the data stack
    IMM(usize),
    PRINT,
}

struct Word {
    name: String,
    inline_iseq: Vec<CompiledInstruction>,
}

#[derive(Copy, Clone)]
enum CompileMode {
    TopLevel,
    Definition,
    DefinitionBody,
}

struct VM {
    data_stack: Vec<usize>,
    call_stack: Vec<usize>,
    instruction_memory: Vec<CompiledInstruction>,
    instruction_pointer: usize,
    compile_mode: CompileMode,
    words: Vec<Word>,
    in_comment: bool,
}

#[derive(Debug, Clone)]
enum VMErr {
    StackUnderflow,
    InvalidToken(String),
}

impl VM {
    fn new() -> VM {
        VM {
            data_stack: vec![],
            call_stack: vec![],
            instruction_memory: Vec::new(),
            instruction_pointer: 0,
            compile_mode: CompileMode::TopLevel,
            in_comment: false,
            words: vec![
                Word {
                    name: String::from("DUP"),
                    inline_iseq: vec![CompiledInstruction::DUP],
                },
                Word {
                    name: String::from("DROP"),
                    inline_iseq: vec![CompiledInstruction::DROP],
                },
                Word {
                    name: String::from("SWAP"),
                    inline_iseq: vec![CompiledInstruction::SWAP],
                },
                Word {
                    name: String::from("+"),
                    inline_iseq: vec![CompiledInstruction::ADD],
                },
                Word {
                    name: String::from("-"),
                    inline_iseq: vec![CompiledInstruction::SUB],
                },
                Word {
                    name: String::from("*"),
                    inline_iseq: vec![CompiledInstruction::MUL],
                },
                Word {
                    name: String::from("/"),
                    inline_iseq: vec![CompiledInstruction::DIV],
                },
                Word {
                    name: String::from("CALL"),
                    inline_iseq: vec![CompiledInstruction::CALL],
                },
                Word {
                    name: String::from("JUMP"),
                    inline_iseq: vec![CompiledInstruction::JUMP],
                },
                Word {
                    name: String::from(";"),
                    inline_iseq: vec![CompiledInstruction::RET],
                },
                Word {
                    name: String::from("."),
                    inline_iseq: vec![CompiledInstruction::PRINT],
                },
            ],
        }
    }

    fn exec_ins(&mut self, ins: CompiledInstruction) -> Result<(), VMErr> {
        match ins {
            CompiledInstruction::NOP => {
                // No operation
            }
            CompiledInstruction::DUP => {
                let tos = self.pop_data_stack()?;
                self.data_stack.push(tos);
                self.data_stack.push(tos);
            }
            CompiledInstruction::DROP => {
                let _tos = self.pop_data_stack()?;
            }
            CompiledInstruction::SWAP => {
                let b = self.pop_data_stack()?;
                let a = self.pop_data_stack()?;
                self.data_stack.push(b);
                self.data_stack.push(a);
            }
            CompiledInstruction::ADD => {
                let b = self.pop_data_stack()?;
                let a = self.pop_data_stack()?;
                self.data_stack.push(a + b);
            }
            CompiledInstruction::SUB => {
                let b = self.pop_data_stack()?;
                let a = self.pop_data_stack()?;
                self.data_stack.push(a - b);
            }
            CompiledInstruction::MUL => {
                let b = self.pop_data_stack()?;
                let a = self.pop_data_stack()?;
                self.data_stack.push(a * b);
            }
            CompiledInstruction::DIV => {
                let b = self.pop_data_stack()?;
                let a = self.pop_data_stack()?;
                self.data_stack.push(a / b);
            }
            CompiledInstruction::IMM(n) => {
                self.data_stack.push(n);
            }
            CompiledInstruction::CALL => {
                self.call_stack.push(self.instruction_pointer);
                self.instruction_pointer = self.pop_data_stack()?;
            }
            CompiledInstruction::JUMP => {
                self.instruction_pointer = self.pop_data_stack()?;
            }
            CompiledInstruction::RET => {
                self.instruction_pointer = self.pop_call_stack()?;
            }
            CompiledInstruction::PRINT => {
                let tos = self.pop_data_stack()?;
                print!(" {}", tos);
            }
        }

        Ok(())
    }

    fn pop_data_stack(&mut self) -> Result<usize, VMErr> {
        match self.data_stack.pop() {
            Some(n) => Ok(n),
            None => Err(VMErr::StackUnderflow),
        }
    }

    fn pop_call_stack(&mut self) -> Result<usize, VMErr> {
        match self.call_stack.pop() {
            Some(n) => Ok(n),
            None => Err(VMErr::StackUnderflow),
        }
    }

    // Places `ins_seq` somewhere in the instruction_memory and execute it.
    pub fn run(&mut self, ins_seq: &[CompiledInstruction]) -> Result<(), VMErr> {
        self.instruction_pointer = self.instruction_memory.len();
        let old_imem_len = self.instruction_memory.len();
        for &ins in ins_seq {
            self.instruction_memory.push(ins);
        }

        loop {
            if self.instruction_pointer == self.instruction_memory.len() {
                // restore original instruction memory
                self.instruction_memory.truncate(old_imem_len);
                return Ok(());
            }

            let ins = *self.instruction_memory
                .get(self.instruction_pointer)
                .unwrap(); // XXX
            self.instruction_pointer += 1;

            if let Err(err) = self.exec_ins(ins) {
                // restore original instruction memory
                self.instruction_memory.truncate(old_imem_len);
                return Err(err);
            }
        }
    }

    pub fn in_compile_mode(&self) -> bool {
        match self.compile_mode {
            CompileMode::TopLevel => false,
            CompileMode::Definition => true,
            CompileMode::DefinitionBody => true,
        }
    }

    // Compiles `line` into a sequence of instructions which is appended to `ins_seq`.
    // As a side-effect, when a ":" definition is occured, this will add a
    // word to the dictionary.
    pub fn compile_line(
        &mut self,
        line: &str,
        ins_seq: &mut Vec<CompiledInstruction>,
    ) -> Result<(), VMErr> {
        let mut remainder: &str = line;

        loop {
            match remainder.find(char::is_whitespace) {
                None => {
                    if remainder.len() > 0 {
                        let _ = self.compile_token(remainder, ins_seq)?;
                    }
                    return Ok(());
                }
                Some(pos) => {
                    if pos > 0 {
                        // if pos == 0, then we found a whitespace at the beginning
                        let (token, rest) = remainder.split_at(pos);
                        let _ = self.compile_token(token, ins_seq)?;
                        remainder = rest;
                    } else {
                        let (_token, rest) = remainder.split_at(1);
                        remainder = rest;
                    }
                }
            }
        }
    }

    fn compile_token(
        &mut self,
        token: &str,
        ins_seq: &mut Vec<CompiledInstruction>,
    ) -> Result<(), VMErr> {
        // process comments
        if self.in_comment {
            if token == ")" {
                self.in_comment = false;
            }
            return Ok(());
        } else {
            if token == "(" {
                self.in_comment = true;
                return Ok(());
            }
        }

        let compile_mode = self.compile_mode;
        match compile_mode {
            CompileMode::TopLevel => {
                match token {
                    ":" => {
                        // starts a definition
                        self.compile_mode = CompileMode::Definition;
                    }
                    _ => {
                        for ins in self.token_to_instruction_seq(token)? {
                            ins_seq.push(ins);
                        }
                    }
                }
            }
            CompileMode::Definition => {
                self.words.push(Word {
                    name: token.into(),
                    inline_iseq: vec![
                        CompiledInstruction::IMM(self.instruction_memory.len()),
                        CompiledInstruction::CALL,
                    ],
                });
                self.compile_mode = CompileMode::DefinitionBody;
            }
            CompileMode::DefinitionBody => {
                for ins in self.token_to_instruction_seq(token)? {
                    self.instruction_memory.push(ins);
                }
                if token == ";" {
                    // ends a definition
                    self.compile_mode = CompileMode::TopLevel;
                }
            }
        }
        Ok(())
    }

    fn token_to_instruction_seq(&self, token: &str) -> Result<Vec<CompiledInstruction>, VMErr> {
        match self.lookup_word(token) {
            None => {
                // it's not a word. it might be a number, or an invalid token
                match usize::from_str(token) {
                    Ok(num) => {
                        return Ok(vec![CompiledInstruction::IMM(num)]);
                    }
                    Err(_) => {
                        return Err(VMErr::InvalidToken(token.into()));
                    }
                }
            }
            Some(word) => {
                return Ok(word.inline_iseq.clone());
            }
        }
    }

    fn lookup_word(&self, token: &str) -> Option<&Word> {
        self.words.iter().find(|w| w.name == token)
    }
}

fn read_line() -> String {
    let stdin = io::stdin();
    let mut iterator = stdin.lock().lines();
    return iterator.next().unwrap().unwrap();
}

fn main() {
    println!("MiniForth started");
    let mut vm = VM::new();
    let mut ins_seq = Vec::new();
    loop {
        let line = read_line();
        ins_seq.clear();
        match vm.compile_line(&line, &mut ins_seq) {
            Ok(()) => {
                match vm.run(&ins_seq) {
                    Ok(()) => {
                        if vm.in_compile_mode() {
                            println!(" compiled");
                        } else {
                            println!(" ok");
                        }
                    }
                    Err(err) => {
                        println!("Error: {:?}", err);
                    }
                }
            }
            Err(err) => {
                println!("Error: {:?}", err);
            }
        }
    }
}
