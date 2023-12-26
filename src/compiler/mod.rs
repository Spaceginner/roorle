use std::{fmt, collections::HashMap};
use crate::syntax::parser::{Script, Token, Value};

mod helper {
    use crate::syntax::parser::Value;

    pub fn value_name(v: &Value) -> &'static str {
        match v {
            Value::Fraction { .. } => "fraction",
            Value::String(..) => "string",
            Value::Whole(..) => "whole",
        }
    }
}

const A_4_FREQUENCY: f64 = 440.0;
const A_4_ABSOLUTE_NOTE: i8 = 57;

pub struct Program(Vec<Instruction>);


impl Program {
    pub fn get_instructions(&self) -> &[Instruction] {
        &self.0
    }
}


impl fmt::Display for Program {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for instr in self.0[..self.0.len() - 1].iter() {
            writeln!(f, "{instr}")?;
        };

        write!(f, "{}", self.0.last().unwrap())
    }
}


struct Scope {
    pub name: Option<String>,
    pub range: (usize, usize),
    pub properties: HashMap<String, Value>,
}


fn parse_octave(v: Option<&Value>) -> Result<u32, CompilingError> {
    match v {
        None => Ok(4),
        Some(Value::Whole(n)) => Ok(*n),
        Some(v) => Err(CompilingError::ValueTypeError {
            pos: None,
            expected: "whole",
            got: helper::value_name(v)
        }),
    }
}

fn parse_bpm(v: Option<&Value>) -> Result<f64, CompilingError> {
    match v {
        None => Err(CompilingError::MissingGlobalProperty { missing: "bpm" }),
        Some(Value::Whole(n)) => {
            if *n < 1 {
                Err(CompilingError::ValueOutOfRange { allowed: (Some(1), None), got: *n, pos: None })
            } else {
                Ok(*n as f64)
            }
        },
        Some(Value::Fraction { numerator, denominator }) => {
            if *numerator == 0 {
                Err(CompilingError::ValueOutOfRange { allowed: (Some(1), None), got: *numerator, pos: None })
            } else {
                Ok(*numerator as f64 / *denominator as f64)
            }
        },
        Some(Value::String(..)) => {
            Err(CompilingError::ValueTypeError { pos: None, expected: "number-like", got: "string" })
        }
    }
}

fn parse_duration(v: &Value) -> Result<f64, CompilingError> {
    match v {
        Value::Whole(n) => {
            Ok(*n as f64)
        },
        Value::Fraction { numerator, denominator } => {
            Ok(*numerator as f64 / *denominator as f64)
        },
        Value::String(..) => {
            Err(CompilingError::ValueTypeError { pos: None, expected: "number-like", got: "string" })
        }
    }
}

fn calculate_frequency(note: i8, octave: u32) -> f64 {
    if note == 9 && octave == 4 {
        A_4_FREQUENCY
    } else {
        let note_absolute = octave as i32 * 12 + note as i32;

        let note_delta = note_absolute - A_4_ABSOLUTE_NOTE as i32;

        let delta = 2.0_f64.powf(note_delta as f64 / 12.0);

        A_4_FREQUENCY * delta
    }
}


fn parse_frequency(note: &str, octave: u32, pos: usize) -> Result<f64, CompilingError> {
    match note {
        "Ces"         => Ok(calculate_frequency(-1, octave)),
        "C"           => Ok(calculate_frequency(0, octave)),
        "Cas" | "Des" => Ok(calculate_frequency(1, octave)),
        "D"           => Ok(calculate_frequency(2, octave)),
        "Das" | "Ees" => Ok(calculate_frequency(3, octave)),
        "E"   | "Fes" => Ok(calculate_frequency(4, octave)),
        "F"   | "Eas" => Ok(calculate_frequency(5, octave)),
        "Fas" | "Ges" => Ok(calculate_frequency(6, octave)),
        "G"           => Ok(calculate_frequency(7, octave)),
        "Gas" | "Aes" => Ok(calculate_frequency(8, octave)),
        "A"           => Ok(calculate_frequency(9, octave)),
        "As"  | "Bes" => Ok(calculate_frequency(10, octave)),
        "B"           => Ok(calculate_frequency(11, octave)),
        "Bas"         => Ok(calculate_frequency(12, octave)),

        unknown_note => Err(CompilingError::UnknownNote { pos, got: unknown_note.into() }),
    }
}


fn compile_note(note: &str, octave: u32, bpm: f64, arguments: &[Value], pos: usize) -> Result<Vec<Instruction>, CompilingError> {
    let frequencies = {
        let mut frequencies = Vec::new();

        frequencies.push(parse_frequency(note, octave, pos)?);

        let got_arguments = arguments.len();
        if got_arguments < 1 {
            return Err(CompilingError::WrongAmountArguments { pos, expected: 1, got: got_arguments })
        }

        for arg in arguments[..arguments.len() - 1].iter() {
            match arg {
                Value::String(additional_note) => frequencies.push(parse_frequency(additional_note, octave, pos)?),
                v => return Err(CompilingError::ValueTypeError { pos: Some(pos), got: helper::value_name(v), expected: "string" })
            };
        };

        frequencies
    };


    let expected_arguments_count = frequencies.len();
    let arguments_count = arguments.len();
    if arguments_count != expected_arguments_count {
        Err(CompilingError::WrongAmountArguments { pos, expected: expected_arguments_count, got: arguments_count })
    } else {
        let duration = bpm / 60.0 * parse_duration(arguments.last().unwrap())?;

        Ok({
            let mut instructions = Vec::new();

            for frequency in frequencies.iter().cloned() {
                instructions.push(Instruction { pos, data: InstructionData::Play { frequency, duration } })
            };

            instructions.push(Instruction { pos, data: InstructionData::Advance { duration } });

            instructions
        })
    }
}


fn compile_goto(name: Option<&str>, pos: Option<usize>, scopes: &[Scope], global_octave: u32, global_bpm: f64, tokens: &[Token], stack: &[&str]) -> Result<Vec<Instruction>, CompilingError> {
    macro_rules! get_from_scope {
        ($scope:ident, $name:literal, $parser:ident, $global:ident) => { $scope.properties.get($name).map(|local| $parser(Some(local))).unwrap_or(Ok($global))? };
    }

    match scopes.iter().find(|s| s.name.as_ref().is_some_and(|s| s == name.unwrap_or("main"))) {
        None => Err(if let Some(name) = name { CompilingError::LabelNotFound { pos: pos.unwrap(), name: String::from(name) } } else { CompilingError::NoMain }),
        Some(scope) => {
            let bpm = get_from_scope!(scope, "bpm", parse_bpm, global_bpm);
            let octave = get_from_scope!(scope, "octave", parse_octave, global_octave);

            let mut instructions = Vec::new();
            for (pos, token) in tokens[scope.range.0..scope.range.1].iter().enumerate() {
                let adapted_pos = pos + scope.range.0;

                if let Token::Command { name, arguments } = token {
                    let name = name.as_str();

                    let mut exiting = false;
                    instructions.append(&mut match name {
                        note @
                        ("Ces" | "C" | "Cas" |
                        "Des" | "D" | "Das" |
                        "Es" | "E" | "Eas" |
                        "Fes" | "F" | "Fas" |
                        "Ges" | "G" | "Gas" |
                        "Aes" | "A" | "As" |
                        "Bes" | "B" | "Bas") => compile_note(note, octave, bpm, arguments, adapted_pos)?,

                        "goto" => {
                            let arguments_len = arguments.len();
                            if arguments_len != 1 {
                                return Err(CompilingError::WrongAmountArguments { pos: adapted_pos, expected: 1, got: arguments_len });
                            };

                            let label = match arguments.get(0).unwrap() {
                                Value::String(name) => name.as_str(),
                                v => return Err(CompilingError::ValueTypeError { pos: Some(adapted_pos), expected: "string", got: helper::value_name(v) }),
                            };

                            let scope_name = scope.name.as_ref().unwrap().as_str();
                            if stack.contains(&scope_name) {
                                return Err(CompilingError::SelfRecursion { pos: adapted_pos })
                            } else {
                                exiting = true;

                                let extended_stack = {
                                    let mut new_stack = Vec::from(stack);
                                    new_stack.push(scope_name);
                                    new_stack
                                };

                                compile_goto(Some(label), Some(adapted_pos), scopes, global_octave, global_bpm, tokens, &extended_stack)?
                            }
                        },

                        "repeat" => {
                            let arguments_len = arguments.len();
                            if arguments_len != 2 {
                                return Err(CompilingError::WrongAmountArguments { pos: adapted_pos, expected: 2, got: arguments_len });
                            };

                            let label = match arguments.get(0).unwrap() {
                                Value::String(name) => name.as_str(),
                                v => return Err(CompilingError::ValueTypeError { pos: Some(adapted_pos), expected: "string", got: helper::value_name(v) }),
                            };

                            let count = match arguments.get(1).unwrap() {
                                Value::Whole(n) => n,
                                v => return Err(CompilingError::ValueTypeError { pos: Some(adapted_pos), expected: "string", got: helper::value_name(v) }),
                            };

                            let mut accum_instructions = Vec::new();
                            let scope_name = scope.name.as_ref().unwrap().as_str();
                            if stack.contains(&scope_name) {
                                exiting = true;
                            } else {
                                let extended_stack = {
                                    let mut new_stack = Vec::from(stack);
                                    new_stack.push(scope_name);
                                    new_stack
                                };

                                for _ in 0..*count {
                                    accum_instructions.append(&mut compile_goto(Some(label), Some(adapted_pos), scopes, global_octave, global_bpm, tokens, &extended_stack)?);
                                }
                            }
                            accum_instructions
                        },

                        _ => return Err(CompilingError::UnknownCommand { pos: adapted_pos, name: String::from(name) }),
                    });

                    if exiting {
                        break;
                    };
                };
            };

            Ok(instructions)
        }
    }
}


impl TryFrom<&Script> for Program {
    type Error = CompilingError;

    fn try_from(script: &Script) -> Result<Self, Self::Error> {
        let scopes = {
            let mut scopes = Vec::new();

            let mut scope_name = None;
            let mut scope_properties = HashMap::new();
            let mut last_ends = 0;

            for (pos, token) in script.get_tokens().iter().enumerate() {
                match token {
                    Token::Label { name } => {
                        scopes.push(Scope {
                            range: (last_ends, pos),
                            name: scope_name,
                            properties: scope_properties,
                        });

                        last_ends = pos;

                        scope_name = Some(name.clone());
                        scope_properties = HashMap::new();
                    },
                    Token::Property { name, value } => {
                        scope_properties.insert(name.clone(), value.clone());
                    },
                    Token::Command { name, .. } => {
                        if scope_name.is_none() {
                            return Err(CompilingError::CommandCalledInGlobal { pos, name: name.clone() });
                        };
                    },
                };
            };

            scopes.push(Scope {
                range: (last_ends, script.get_tokens().len()),
                name: scope_name,
                properties: scope_properties,
            });

            scopes
        };

        let instructions = {
            let global_properties = &scopes.get(0).unwrap().properties;

            let global_octave = parse_octave(global_properties.get("octave"))?;
            let global_bpm = parse_bpm(global_properties.get("bpm"))?;

            compile_goto(None, None, &scopes, global_octave, global_bpm, script.get_tokens(), &[])?
        };

        Ok(Self(instructions))
    }
}


#[derive(Debug)]
pub struct Instruction {
    pub pos: usize,
    pub data: InstructionData,
}

#[derive(Debug)]
pub enum InstructionData {
    Advance {
        duration: f64,
    },
    Play {
        frequency: f64,
        duration: f64,
    },
}


impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.pos + 1)?;

        match self.data {
            InstructionData::Play { frequency, duration } => write!(f, "play {frequency:.2}Hz {duration:.5}s"),
            InstructionData::Advance { duration } => write!(f, "advance {duration:.5}s"),
        }
    }
}


#[derive(Debug)]
pub enum CompilingError {
    MissingGlobalProperty {
        missing: &'static str,
    },
    ValueTypeError {
        expected: &'static str,
        got: &'static str,
        pos: Option<usize>,
    },
    ValueOutOfRange {
        allowed: (Option<u32>, Option<u32>),
        got: u32,
        pos: Option<usize>,
    },
    UnknownCommand {
        name: String,
        pos: usize,
    },
    WrongAmountArguments {
        expected: usize,
        got: usize,
        pos: usize,
    },
    CommandCalledInGlobal {
        name: String,
        pos: usize,
    },
    NoMain,
    LabelNotFound {
        name: String,
        pos: usize,
    },
    SelfRecursion {
        pos: usize,
    },
    UnknownNote {
        pos: usize,
        got: String,
    },
}
