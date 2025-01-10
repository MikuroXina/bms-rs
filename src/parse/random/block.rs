use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) enum ControlFlowBlock {
    Random(RandomBlock),
    Switch(SwitchBlock),
}

impl ControlFlowBlock {
    pub fn pass(&self) -> bool {
        match self {
            ControlFlowBlock::Random(random_block) => random_block.pass(),
            ControlFlowBlock::Switch(switch_block) => switch_block.pass(),
        }
    }
    pub fn is_in_if_block(&self) -> bool {
        match self {
            ControlFlowBlock::Random(random_block) => random_block.is_in_if_block(),
            ControlFlowBlock::Switch(switch_block) => switch_block.is_in_case_block(),
        }
    }
}

impl From<RandomBlock> for ControlFlowBlock {
    fn from(block: RandomBlock) -> Self {
        ControlFlowBlock::Random(block)
    }
}

impl From<SwitchBlock> for ControlFlowBlock {
    fn from(block: SwitchBlock) -> Self {
        ControlFlowBlock::Switch(block)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct RandomBlock {
    /// If the parent block is not passed, set it to None.
    random_value: Option<u32>,
    /// Check if the if values have duplication.
    if_values: BTreeSet<u32>,
    /// If Value/Loglc matches in this floor now.
    pass: bool,
    /// If the pass value has been set to true.
    has_passed: bool,
    /// If there is an else added.
    is_in_else: bool,
}

impl RandomBlock {
    pub fn new(random_value: Option<u32>) -> Self {
        Self {
            random_value,
            if_values: BTreeSet::new(),
            is_in_else: false,
            has_passed: false,
            pass: false,
        }
    }
    /// Return true if it is not already exist.
    pub fn add_if_value(&mut self, if_value: u32) -> bool {
        if let Some(random_value) = self.random_value {
            self.pass |= if_value == random_value;
            self.has_passed |= if_value == random_value;
        }
        self.if_values.insert(if_value)
    }
    pub fn clear_if_values(&mut self) {
        self.pass = false;
        self.is_in_else = false;
        self.if_values.clear();
    }
    pub fn is_if_value_empty(&self) -> bool {
        self.if_values.is_empty()
    }
    /// Return true if other command can pass.
    pub fn pass(&self) -> bool {
        self.pass || (self.is_in_else && !self.has_passed)
    }
    /// Reset all the if status
    pub fn reset_if(&mut self) {
        self.pass = false;
        self.is_in_else = false;
        self.if_values.clear();
        self.has_passed = false;
    }
    /// Return if the if_values is clear before.
    pub fn check_clear_and_add_if_value(&mut self, if_value: u32) -> bool {
        // Clear
        let ret = self.is_if_value_empty();
        self.clear_if_values();
        // Add
        self.add_if_value(if_value);
        ret
    }
    /// Return false if there is already an else.
    pub fn add_else(&mut self) -> bool {
        self.clear_if_values();
        self.is_in_else = !self.is_in_else;
        self.is_in_else
    }
    /// Return if there is in else.
    pub fn is_in_else(&self) -> bool {
        self.is_in_else
    }
    pub fn is_in_if_block(&self) -> bool {
        !self.is_if_value_empty() || self.is_in_else()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SwitchBlock {
    /// If the parent block is not passed, set it to None.
    switch_value: Option<u32>,
    /// Check if the if values have duplication.
    case_values: BTreeSet<u32>,
    /// If Value/Loglc matches in this floor now.
    pass: bool,
    /// If the pass value has been set to true.
    has_passed: bool,
    /// If there is an default added.
    is_in_default: bool,
}

impl SwitchBlock {
    pub fn new(switch: Option<u32>) -> Self {
        Self {
            switch_value: switch,
            case_values: BTreeSet::new(),
            is_in_default: false,
            has_passed: false,
            pass: false,
        }
    }
    /// Return true if it is not already exist.
    pub fn add_case_value(&mut self, case_value: u32) -> bool {
        if let Some(switch_value) = self.switch_value {
            self.pass |= case_value == switch_value;
            self.has_passed |= case_value == switch_value;
        }
        self.case_values.insert(case_value)
    }
    pub fn clear_case_values(&mut self) {
        self.pass = false;
        self.is_in_default = false;
        self.case_values.clear();
    }
    pub fn is_case_value_empty(&self) -> bool {
        self.case_values.is_empty()
    }
    /// Return true if other command can pass.
    pub fn pass(&self) -> bool {
        self.pass || (self.is_in_default && !self.has_passed)
    }
    /// Return false if there is already an else.
    pub fn add_default(&mut self) -> bool {
        self.clear_case_values();
        self.is_in_default = !self.is_in_default;
        self.is_in_default
    }
    /// Return if there is in else.
    pub fn is_in_default(&self) -> bool {
        self.is_in_default
    }
    pub fn is_in_case_block(&self) -> bool {
        !self.is_case_value_empty() || self.is_in_default()
    }
}
