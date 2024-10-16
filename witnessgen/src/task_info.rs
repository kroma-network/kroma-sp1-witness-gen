use alloy_primitives::B256;

#[derive(Clone, Debug)]
pub struct TaskInfo {
    l2_hash: B256,
    l1_head_hash: B256,
}

impl Default for TaskInfo {
    fn default() -> Self {
        Self { l2_hash: B256::default(), l1_head_hash: B256::default() }
    }
}

impl TaskInfo {
    pub fn set(&mut self, l2_hash: B256, l1_head_hash: B256) {
        self.l2_hash = l2_hash;
        self.l1_head_hash = l1_head_hash;
    }

    pub fn release(&mut self) {
        self.l2_hash = B256::default();
        self.l1_head_hash = B256::default();
    }

    pub fn is_equal(&self, l2_hash: B256, l1_head_hash: B256) -> bool {
        if self.l2_hash == l2_hash && self.l1_head_hash == l1_head_hash {
            true
        } else {
            false
        }
    }

    pub fn is_empty(&self) -> bool {
        let default_value = B256::default();
        if self.l2_hash == default_value && self.l1_head_hash == default_value {
            true
        } else {
            false
        }
    }
}
