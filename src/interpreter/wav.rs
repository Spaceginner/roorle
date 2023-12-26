use std::collections::LinkedList;
use crate::compiler::{Instruction, InstructionData, Program};


#[derive(Clone, Debug)]
struct Sound {
    pub frequency: f64,
    pub started_at: f64,
    pub ends_at: f64,
    pub volume: f64,
}


impl Sound {
    pub fn get_sine_value_at(&self, seconds: f64) -> f64 {
        (seconds * 2.0 * std::f64::consts::PI * self.frequency /* - self.started_at */).sin() * self.volume
    }
}


#[derive(Copy, Clone, Debug)]
pub enum SampleSize {
    Small = 8,
    Large = 16,
}


pub fn interpret(program: &Program, sample_rate: u32, sample_size: SampleSize) -> Vec<u8> {
    let mut samples = {
        let mut samples = Vec::<u8>::new();

        let mut sounds_pull = LinkedList::new();
        let mut samples_stepped = 0_u32;
        for instruction in program.get_instructions().iter() {
            match instruction.data {
                InstructionData::Play { frequency, duration } => {
                    let seconds_passed = samples_stepped as f64 / sample_rate as f64;

                    sounds_pull.push_back(Sound {
                        frequency,
                        started_at: seconds_passed,
                        ends_at: seconds_passed + duration,
                        volume: 1.0,
                    });
                },
                InstructionData::Advance { duration } => {
                    let samples_to_compute = (duration * sample_rate as f64).round() as u32;

                    for _ in 0..samples_to_compute {
                        samples_stepped += 1;

                        let seconds_passed = samples_stepped as f64 / sample_rate as f64;

                        for (i, sound) in sounds_pull.clone().iter().enumerate() {
                            if sound.ends_at < seconds_passed {
                                sounds_pull.remove(i);
                            };
                        };

                        let values = sounds_pull.iter().map(|s| s.get_sine_value_at(seconds_passed)).collect::<Vec<_>>();
                        let value = values.iter().sum::<f64>() / values.len() as f64;

                        samples.append(&mut match sample_size {
                            SampleSize::Small => ((i8::MAX as f64 * value).round() as u8 + i8::MAX as u8).to_le_bytes().to_vec(),
                            SampleSize::Large => ((i16::MAX as f64 * value).round() as i16).to_le_bytes().to_vec(),
                        });
                    };
                },
            }
        };

        samples
    };

    {
        let mut buffer = Vec::new();

        buffer.append(&mut b"RIFF".to_vec());
        buffer.append(&mut (36 + samples.len() as u32).to_le_bytes().to_vec());
        buffer.append(&mut b"WAVE".to_vec());
        buffer.append(&mut b"fmt\x20".to_vec());
        buffer.append(&mut 16_u32.to_le_bytes().to_vec());
        buffer.append(&mut 1_u16.to_le_bytes().to_vec());
        buffer.append(&mut 1_u16.to_le_bytes().to_vec());
        buffer.append(&mut sample_rate.to_le_bytes().to_vec());
        buffer.append(&mut (sample_rate * sample_size as u32 / 8).to_le_bytes().to_vec());
        buffer.append(&mut (sample_size as u16 / 8).to_le_bytes().to_vec());
        buffer.append(&mut (sample_size as u16).to_le_bytes().to_vec());
        buffer.append(&mut b"data".to_vec());
        buffer.append(&mut (samples.len() as u32).to_le_bytes().to_vec());
        buffer.append(&mut samples);

        buffer
    }
}
