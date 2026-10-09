/*
Next-Word
BY: StealthyFi
*/

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::cmp::min;

// Pseudo-Random Number Generator (XorShift64)
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: if seed == 0 { 1 } else { seed } }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn next_u32(&mut self, max: u32) -> u32 {
        if max == 0 { return 0; }
        (self.next_u64() % max as u64) as u32
    }
}

// Minimal JSON Parser (No external crates)
#[derive(Debug, Clone)]
#[allow(dead_code)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> JsonParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn parse(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() {
            return Err("Unexpected end of JSON".into());
        }
        self.parse_value()
    }

    fn parse_value(&mut self) -> Result<JsonValue, String> {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() { return Err("Unexpected end of JSON".into()); }

        match self.bytes[self.pos] {
            b'"' => self.parse_string().map(JsonValue::Str),
            b'{' => self.parse_object().map(JsonValue::Object),
            b'[' => self.parse_array().map(JsonValue::Array),
            b't' | b'f' => self.parse_bool(),
            b'n' => self.parse_null(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            c => Err(format!("Unexpected character: {}", c as char)),
        }
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect(b'"')?;
        let mut s = String::new();

        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b'"' {
                self.pos += 1;
                return Ok(s);
            }
            if c == b'\\' {
                self.pos += 1;
                if self.pos >= self.bytes.len() { return Err("Unexpected end of string escape".into()); }
                let esc = self.bytes[self.pos];
                match esc {
                    b'"' => s.push('"'),
                    b'\\' => s.push('\\'),
                    b'/' => s.push('/'),
                    b'n' => s.push('\n'),
                    b'r' => s.push('\r'),
                    b't' => s.push('\t'),
                    _ => return Err(format!("Invalid escape sequence: \\{}", esc as char)),
                }
            } else {
                let mut end = self.pos + 1;
                while end < self.bytes.len() && self.bytes[end] != b'"' && self.bytes[end] != b'\\' {
                    end += 1;
                }
                let slice = &self.bytes[self.pos..end];
                s.push_str(std::str::from_utf8(slice).map_err(|_| "Invalid UTF-8 in string")?);
                self.pos = end;
                continue;
            }
            self.pos += 1;
        }
        Err("Unterminated string".into())
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        if self.bytes[self.pos] == b'-' { self.pos += 1; }
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() { self.pos += 1; }
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'.' {
            self.pos += 1;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() { self.pos += 1; }
        }
        let num_str = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap();
        num_str.parse::<f64>().map(JsonValue::Number).map_err(|_| "Invalid number".into())
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.bytes[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.bytes[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err("Invalid boolean".into())
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.bytes[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err("Invalid null".into())
        }
    }

    fn parse_array(&mut self) -> Result<Vec<JsonValue>, String> {
        self.expect(b'[')?;
        let mut arr = Vec::new();
        self.skip_whitespace();
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b']' {
            self.pos += 1;
            return Ok(arr);
        }
        loop {
            arr.push(self.parse_value()?);
            self.skip_whitespace();
            if self.pos >= self.bytes.len() { return Err("Unterminated array".into()); }
            if self.bytes[self.pos] == b']' {
                self.pos += 1;
                return Ok(arr);
            }
            self.expect(b',')?;
        }
    }

    fn parse_object(&mut self) -> Result<Vec<(String, JsonValue)>, String> {
        self.expect(b'{')?;
        let mut obj = Vec::new();
        self.skip_whitespace();
        if self.pos < self.bytes.len() && self.bytes[self.pos] == b'}' {
            self.pos += 1;
            return Ok(obj);
        }
        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':')?;
            let val = self.parse_value()?;
            obj.push((key, val));
            self.skip_whitespace();
            if self.pos >= self.bytes.len() { return Err("Unterminated object".into()); }
            if self.bytes[self.pos] == b'}' {
                self.pos += 1;
                return Ok(obj);
            }
            self.expect(b',')?;
        }
    }

    fn expect(&mut self, expected: u8) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != expected {
            return Err(format!("Expected '{}'", expected as char));
        }
        self.pos += 1;
        Ok(())
    }
}

fn parse_json(content: &str) -> Result<JsonValue, String> {
    let mut parser = JsonParser::new(content.as_bytes());
    parser.parse()
}

// Configuration
struct Config {
    context_size: usize,
    predict_count: usize,
    dataset_path: String,
    model_path: String,
}

fn load_config(path: &str) -> Config {
    let content = fs::read_to_string(path).expect("Failed to read Config.json");
    let json = parse_json(&content).expect("Failed to parse Config.json");

    if let JsonValue::Object(obj) = json {
        let mut ctx_size = 5;
        let mut predict_count = 1;
        let mut ds_path = "Dataset.json".to_string();
        let mut mod_path = "Model.bin".to_string();

        for (k, v) in obj {
            match k.as_str() {
                "context_size" => if let JsonValue::Number(n) = v { ctx_size = n as usize; },
                "predict_count" => if let JsonValue::Number(n) = v { predict_count = n as usize; },
                "dataset_path" => if let JsonValue::Str(s) = v { ds_path = s; },
                "model_path" => if let JsonValue::Str(s) = v { mod_path = s; },
                _ => {}
            }
        }
        Config {
            context_size: ctx_size,
            predict_count: if predict_count == 0 { 1 } else { predict_count },
            dataset_path: ds_path,
            model_path: mod_path,
        }
    } else {
        panic!("Config.json must be an object");
    }
}

// Markov Chain Model
struct MarkovModel {
    context_size: usize,
    transitions: HashMap<String, Vec<(String, u32)>>,
    sentence_count: usize,
}

impl MarkovModel {
    fn new(context_size: usize) -> Self {
        Self {
            context_size,
            transitions: HashMap::new(),
            sentence_count: 0,
        }
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.split_whitespace()
            .map(|w| {
                w.trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
                    .to_lowercase()
            })
            .filter(|w| !w.is_empty())
            .collect()
    }

    fn train(&mut self, text: &str) {
        let tokens = Self::tokenize(text);
        self.sentence_count += 1;

        for i in 0..tokens.len() {
            let max_context = min(i, self.context_size);
            for ctx_len in 1..=max_context {
                let start = i - ctx_len;
                let context = tokens[start..i].join(" ");
                let next_word = &tokens[i];

                let transitions = self.transitions.entry(context).or_insert_with(Vec::new);
                if let Some((_, count)) = transitions.iter_mut().find(|(w, _)| w == next_word) {
                    *count += 1;
                } else {
                    transitions.push((next_word.clone(), 1));
                }
            }
        }
    }

    fn predict(&self, input: &str, rng: &mut Rng) -> Option<String> {
        let tokens = Self::tokenize(input);
        if tokens.is_empty() {
            return None;
        }

        for ctx_len in (1..=self.context_size.min(tokens.len())).rev() {
            let start = tokens.len() - ctx_len;
            let context = tokens[start..].join(" ");

            if let Some(transitions) = self.transitions.get(&context) {
                let total_weight: u32 = transitions.iter().map(|(_, w)| w).sum();
                if total_weight == 0 { continue; }

                let mut roll = rng.next_u32(total_weight);
                for (word, weight) in transitions {
                    if roll < *weight {
                        return Some(word.clone());
                    }
                    roll -= *weight;
                }
            }
        }
        None
    }

    fn get_stats(&self) -> (usize, usize, usize) {
        let unique_contexts = self.transitions.len();
        let total_transitions: usize = self.transitions.values()
            .map(|v| v.len())
            .sum();
        (self.sentence_count, unique_contexts, total_transitions)
    }

    fn save(&self, path: &str) -> io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(b"NWMD")?;
        file.write_all(&1u32.to_le_bytes())?;
        file.write_all(&(self.context_size as u32).to_le_bytes())?;
        file.write_all(&(self.sentence_count as u32).to_le_bytes())?;
        file.write_all(&(self.transitions.len() as u32).to_le_bytes())?;

        for (ctx, trans) in &self.transitions {
            Self::write_string(&mut file, ctx)?;
            file.write_all(&(trans.len() as u32).to_le_bytes())?;
            for (word, count) in trans {
                Self::write_string(&mut file, word)?;
                file.write_all(&count.to_le_bytes())?;
            }
        }
        Ok(())
    }

    fn load(path: &str) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;
        if &magic != b"NWMD" { return Err(io::Error::new(io::ErrorKind::InvalidData, "Invalid model magic")); }

        let mut version = [0u8; 4];
        file.read_exact(&mut version)?;
        if u32::from_le_bytes(version) != 1 { return Err(io::Error::new(io::ErrorKind::InvalidData, "Unsupported model version")); }

        let mut ctx_size_buf = [0u8; 4];
        file.read_exact(&mut ctx_size_buf)?;
        let context_size = u32::from_le_bytes(ctx_size_buf) as usize;

        let mut sentence_count_buf = [0u8; 4];
        file.read_exact(&mut sentence_count_buf)?;
        let sentence_count = u32::from_le_bytes(sentence_count_buf) as usize;

        let mut num_ctx_buf = [0u8; 4];
        file.read_exact(&mut num_ctx_buf)?;
        let num_ctx = u32::from_le_bytes(num_ctx_buf) as usize;

        let mut transitions = HashMap::with_capacity(num_ctx);

        for _ in 0..num_ctx {
            let ctx = Self::read_string(&mut file)?;
            let mut num_trans_buf = [0u8; 4];
            file.read_exact(&mut num_trans_buf)?;
            let num_trans = u32::from_le_bytes(num_trans_buf) as usize;

            let mut trans = Vec::with_capacity(num_trans);
            for _ in 0..num_trans {
                let word = Self::read_string(&mut file)?;
                let mut count_buf = [0u8; 4];
                file.read_exact(&mut count_buf)?;
                let count = u32::from_le_bytes(count_buf);
                trans.push((word, count));
            }
            transitions.insert(ctx, trans);
        }

        Ok(Self { context_size, transitions, sentence_count })
    }

    fn write_string(file: &mut File, s: &str) -> io::Result<()> {
        let bytes = s.as_bytes();
        file.write_all(&(bytes.len() as u32).to_le_bytes())?;
        file.write_all(bytes)?;
        Ok(())
    }

    fn read_string(file: &mut File) -> io::Result<String> {
        let mut len_buf = [0u8; 4];
        file.read_exact(&mut len_buf)?;
        let len = u32::from_le_bytes(len_buf) as usize;
        let mut bytes = vec![0u8; len];
        file.read_exact(&mut bytes)?;
        String::from_utf8(bytes).map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8 in model"))
    }
}

// Main Execution
fn main() {
    println!("Loading configuration...");
    let config = load_config("Config.json");

    let model = if File::open(&config.model_path).is_ok() {
        println!("Loading existing model from {}...", config.model_path);
        MarkovModel::load(&config.model_path).expect("Failed to load model")
    } else {
        println!("Model not found. Training new model from {}...", config.dataset_path);
        let dataset_content = fs::read_to_string(&config.dataset_path).expect("Failed to read Dataset.json");
        let dataset_json = parse_json(&dataset_content).expect("Failed to parse Dataset.json");

        let mut model = MarkovModel::new(config.context_size);

        if let JsonValue::Array(arr) = dataset_json {
            for item in arr {
                if let JsonValue::Str(text) = item {
                    model.train(&text);
                }
            }
        } else {
            panic!("Dataset.json must be an array of strings");
        }

        println!("Saving model to {}...", config.model_path);
        model.save(&config.model_path).expect("Failed to save model");
        model
    };

    let (sentences, contexts, transitions) = model.get_stats();
    println!("Data: {}", sentences);
    //println!("Unique contexts: {}", contexts);
    //println!("Total transitions: {}", transitions);
    //println!("Context size: {}", model.context_size);
    //println!("Predict count: {}", config.predict_count);

    println!("Type your text and press Enter to predict the next word(s). Type 'exit' to quit.\n");

    let mut rng = Rng::new(42);
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();
        input.clear();

        if stdin.read_line(&mut input).is_err() {
            break;
        }

        let trimmed = input.trim();
        if trimmed.eq_ignore_ascii_case("exit") {
            break;
        }
        if trimmed.is_empty() {
            continue;
        }

        let mut current_context = trimmed.to_string();
        let mut predictions: Vec<String> = Vec::with_capacity(config.predict_count);

        for _ in 0..config.predict_count {
            match model.predict(&current_context, &mut rng) {
                Some(word) => {
                    predictions.push(word.clone());
                    current_context = format!("{} {}", current_context, word);
                }
                None => break,
            }
        }

        if predictions.is_empty() {
            println!("Prediction: [No suitable context found]\n");
        } else {
            println!("Prediction: {}\n", predictions.join(" "));
        }
    }
}