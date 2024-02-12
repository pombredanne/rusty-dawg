// Stats for logging during the building of a DAWG or CDAWG.

use anyhow::Result;
use std::path::Path;
use std::fs::OpenOptions;
use std::io::prelude::*;
use serde::{Deserialize, Serialize};

use cdawg::Cdawg;
use graph::indexing::IndexType;
use cdawg::cdawg_edge_weight::CdawgEdgeWeight;
use weight::Weight;
use graph::memory_backing::MemoryBacking;

#[derive(Serialize, Deserialize)]
pub struct BuildStats {
    pub n_tokens: usize,
    pub n_nodes: usize,
    pub n_edges: usize,
    pub n_bytes: u64,
    pub balance_ratio: f64,
    pub elapsed_time: f32,
}

impl BuildStats {
    pub fn from_cdawg<W, Ix, Mb>(cdawg: &Cdawg<W, Ix, Mb>, n_tokens: usize, n_bytes: u64, elapsed_time: f32) -> Self
    where
        W: Weight + Serialize + for<'de> Deserialize<'de> + Clone,
        Ix: IndexType,
        Mb: MemoryBacking<W, CdawgEdgeWeight<Ix>, Ix>,
    {
        Self {
            n_tokens,
            n_nodes: cdawg.node_count(),
            n_edges: cdawg.edge_count(),
            n_bytes: n_bytes,
            balance_ratio: cdawg.balance_ratio(1),
            elapsed_time,
        }
    }

    pub fn get_nodes_per_token(&self) -> f64 {
        (self.n_nodes as f64) / (self.n_tokens as f64)
    }

    pub fn get_edges_per_token(&self) -> f64 {
        (self.n_edges as f64) / (self.n_tokens as f64)
    }

    pub fn get_tokens_per_byte(&self) -> f64 {
        (self.n_tokens as f64) / (self.n_bytes as f64)
    }

    pub fn append_to_jsonl<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let blob = serde_json::to_string(self)?;

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(path)
            .unwrap();

        Ok(writeln!(file, "{}", blob)?)
    }
}