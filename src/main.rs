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
    EVAL,
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
    word_dictionary_start: usize,
    word_dictionary_current: usize,
    interpreter_next_ins: usize,
    compile_mode: CompileMode,
    words: Vec<Word>,
    in_comment: bool,
}

#[derive(Debug, Copy, Clone)]
enum VMErr {
    StackUnderflow,
    Eval,
}

impl VM {
    fn new() -> VM {
        VM {
            data_stack: vec![],
            call_stack: vec![],
            instruction_memory: (0..1024).map(|_| CompiledInstruction::NOP).collect(),
            instruction_pointer: 1024,
            // This is from where the dictionary is build
            word_dictionary_start: 256,
            word_dictionary_current: 256,
            // this is where we put code when we directly execute code.
            interpreter_next_ins: 0,
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
            CompiledInstruction::EVAL => return Err(VMErr::Eval),
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


    fn run(&mut self) -> Result<(), VMErr> {
        loop {
            let ins = *self.instruction_memory
                .get(self.instruction_pointer)
                .unwrap();
            self.instruction_pointer += 1;

            self.exec_ins(ins)?;
        }
    }

    fn in_compile_mode(&self) -> bool {
        match self.compile_mode {
            CompileMode::TopLevel => false,
            CompileMode::Definition => true,
            CompileMode::DefinitionBody => true,
        }
    }

    fn eval_line(&mut self, line: &str) {
        // reset
        self.interpreter_next_ins = 0;

        let mut remainder: &str = line;

        loop {
            match remainder.find(char::is_whitespace) {
                None => {
                    if remainder.len() > 0 {
                        self.eval_token(remainder);
                    }
                    break;
                }
                Some(pos) => {
                    if pos > 0 {
                        // if pos == 0, then we found a whitespace at the beginning
                        let (token, rest) = remainder.split_at(pos);
                        self.eval_token(token);
                        remainder = rest;
                    } else {
                        let (_token, rest) = remainder.split_at(1);
                        remainder = rest;
                    }
                }
            }
        }

        self.push_ins(CompiledInstruction::EVAL);
        self.instruction_pointer = 0;
    }

    fn eval_token(&mut self, token: &str) {
        // process comments
        if self.in_comment {
            if token == ")" {
                self.in_comment = false;
            }
            return;
        } else {
            if token == "(" {
                self.in_comment = true;
                return;
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
                        let ins_seq = self.token_to_instruction_seq(token);
                        for ins in ins_seq {
                            self.push_ins(ins);
                        }
                    }
                }
            }
            CompileMode::Definition => {
                self.words.push(Word {
                    name: token.into(),
                    inline_iseq: vec![
                        CompiledInstruction::IMM(self.word_dictionary_current),
                        CompiledInstruction::CALL,
                    ],
                });
                self.compile_mode = CompileMode::DefinitionBody;
            }
            CompileMode::DefinitionBody => {
                let ins_seq = self.token_to_instruction_seq(token);
                for ins in ins_seq {
                    self.push_ins_word(ins);
                }
                if token == ";" {
                    // ends a definition
                    self.compile_mode = CompileMode::TopLevel;
                }
            }
        }
    }

    fn push_ins(&mut self, ins: CompiledInstruction) {
        assert!(self.interpreter_next_ins < self.word_dictionary_start);
        self.instruction_memory[self.interpreter_next_ins] = ins;
        self.interpreter_next_ins += 1;
    }

    fn push_ins_word(&mut self, ins: CompiledInstruction) {
        self.instruction_memory[self.word_dictionary_current] = ins;
        self.word_dictionary_current += 1;
    }

    fn token_to_instruction_seq(&self, token: &str) -> Vec<CompiledInstruction> {
        match self.lookup_word(token) {
            None => {
                // it's not a word. it might be a number, or an invalid token
                match usize::from_str(token) {
                    Ok(num) => {
                        return vec![CompiledInstruction::IMM(num)];
                    }
                    Err(_) => {
                        eprintln!("Invalid token: {}", token);
                        return vec![];
                    }
                }
            }
            Some(word) => {
                return word.inline_iseq.clone();
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
    println!("ToyForth started");
    let mut vm = VM::new();
    loop {
        let line = read_line();
        vm.eval_line(&line);
        match vm.run() {
            Ok(()) => {
            }
            Err(VMErr::Eval) => {
                // return to eval loop
            }
            Err(err) => {
                println!("Error: {:?}", err);
            }
        }
        if vm.in_compile_mode() {
            println!(" compiled");
        } else {
            println!(" ok");
        }
    }
}
