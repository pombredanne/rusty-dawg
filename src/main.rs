// Implementation of Suffix DFA in Rust.
// 
// See here for Graph info:
// https://docs.rs/petgraph/latest/petgraph/graph/struct.Graph.html
// 
// See here for Suffix Automaton algorithm in Python:
// https://github.com/viking-sudo-rm/knn-transformers/blob/master/src/suffix_dfa_builder.py
// 

extern crate petgraph;
extern crate kdam;
extern crate substring;
extern crate serde;
extern crate serde_json;
extern crate bitvec;
extern crate bincode;
extern crate tempfile;

mod dawg;
mod weight;
mod stat_utils;
mod token_index;
mod graph;
mod evaluator;
mod lms;

use lms::LM;
use lms::kn_lm::KNLM;
use lms::induction_lm::InductionLM;

use std::mem::size_of;
use std::fs;
use std::env;
use bincode::serialize_into;

use kdam::tqdm;

use stat_utils::*;
use dawg::Dawg;
use weight::BasicWeight;
use token_index::TokenIndex;
use evaluator::Evaluator;

fn create_lms(lms: &mut Vec<Box<dyn LM>>) {
    for delta in vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9].iter() {
        let maxgram = KNLM::new(format!("maxgram_kn-{}", delta), *delta, -1);
        lms.push(Box::new(maxgram));
        for n in vec![4, 6, 8].iter() {
            let ngram = KNLM::new(format!("{}gram_kn-{}", n, delta), *delta, *n);
            lms.push(Box::new(ngram));
            for induct_delta in vec![0.9, 0.95].iter() {
                let induct_backoff = KNLM::new(format!("sub_{}gram_kn-{}", n, delta), *delta, *n);
                let induct = InductionLM::new(format!("{}gram_kn-{}_induct-{}", n, delta, induct_delta), Box::new(induct_backoff), *induct_delta);
                lms.push(Box::new(induct))
            }
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("sizeof(edge): {}B", size_of::<usize>());
    println!("sizeof(node): {}B", size_of::<BasicWeight>());

    let args: Vec<String> = env::args().collect();
    let train_path = &args[1];
    let test_path = &args[2];
    let save_path = &args[3];
    let results_path = &args[4];
    let gen_path: Option<&str>;
    let gen_results_path: Option<&str>;

    // See if we have a generalization/generated set.
    if args.len() >= 6 {
        gen_path = Some(&args[5]);
        gen_results_path = Some(&args[6]);
    } else {
        gen_path = None;
        gen_results_path = None;
    }

    let train_raw: String = fs::read_to_string(train_path).expect("Error loading train");
    let test_raw: String = fs::read_to_string(test_path).expect("Error loading test");
    let gen_raw: String = match gen_path {
        Some(path) => fs::read_to_string(path).expect("Error loading gen"),
        None => "".to_string(),
    };

    // Load at word level.
    let mut index = TokenIndex::new();
    let train: Vec<usize> = train_raw.split_whitespace().map(|x| index.add(x)).collect();
    let eval_threshold = train.len() / 20;
    println!("#(train): {}", train.len());

    let mut test: Vec<usize> = test_raw.split_whitespace().map(|x| index.add(x)).collect();
    let old_test_len = test.len();
    test = (&test[0..10000]).to_vec();  // Truncate to 10,000 tokens.
    println!("#(test): {}/{}", test.len(), old_test_len);

    let gen: Vec<usize> = gen_raw.split_whitespace().map(|x| index.add(x)).collect();
    println!("#(gen): {}", gen.len());
    println!("#(vocab): {}", index.count);

    let mut lms: Vec<Box<dyn LM>> = Vec::new();
    create_lms(&mut lms);
    let mut evaluator = Evaluator::new(&mut lms, &test);
    let mut gen_lms: Vec<Box<dyn LM>> = Vec::new();
    create_lms(&mut gen_lms);
    let mut gen_evaluator = Evaluator::new(&mut gen_lms, &gen);

    let mut dawg: Dawg<usize> = Dawg::with_capacity(2 * train.len());
    let mut last = dawg.get_initial();
    for (idx, token) in tqdm!(train.iter()).enumerate() {
        last = dawg.extend(*token, last);
        if idx % eval_threshold == 0 && idx != 0 {
            let good_turing = good_turing_estimate(&dawg, train.len());        
            evaluator.evaluate(&dawg, idx, good_turing);
            evaluator.to_json(results_path)?;
            match gen_results_path {
                Some(gen_path) => {
                    gen_evaluator.evaluate(&dawg, idx, good_turing);
                    gen_evaluator.to_json(gen_path);
                },
                None => {},
            }
            
            // checkpoint(&dawg, save_path)?;
        }
    }
    println!("Completed!");
    println!("  Node count: {}", dawg.node_count());
    println!("  Edge count: {}", dawg.edge_count());

    println!("Saving DAWG...");
    checkpoint(&dawg, save_path)?;
    Ok(())
}

fn checkpoint(dawg: &Dawg<usize>, save_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut save_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(save_path)?;
    serialize_into(&save_file, &dawg)?;
    // println!("Successfully saved DAWG to {}!", save_path);

    // HOWTO: Deserialize
    // let mut load_file = fs::OpenOptions::new()
    //     .read(true)
    //     .open(save_path)?;
    // let decoded: Dawg<usize> = deserialize_from(&load_file).expect("Failed to deserialize");
    // println!("decoded DAWG");
    Ok(())
}