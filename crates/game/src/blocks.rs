//! Block definitions and properties

use serde::{Deserialize, Serialize};
use world::BlockID;

/// Block properties loaded from JSON data files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDefinition {
    pub id: BlockID,
    pub name: String,
    pub solid: bool,
    pub transparent: bool,
    pub texture_id: u32,
}

/// Block registry for managing all block types
#[derive(Debug, Default)]
pub struct BlockRegistry {
    blocks: Vec<BlockDefinition>,
}

impl BlockRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn register_block(&mut self, block: BlockDefinition) {
        self.blocks.push(block);
    }
    
    pub fn get_block(&self, id: BlockID) -> Option<&BlockDefinition> {
        self.blocks.iter().find(|b| b.id == id)
    }
    
    pub fn load_from_json(&mut self, json_data: &str) -> anyhow::Result<()> {
        let blocks: Vec<BlockDefinition> = serde_json::from_str(json_data)?;
        for block in blocks {
            self.register_block(block);
        }
        Ok(())
    }
}