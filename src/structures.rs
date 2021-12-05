#[derive(Debug, Clone)]
pub struct Component {
    /// The number of slots in the logical memory
    size: u64,
    /// Bitwidth of the stored data elements
    width: u64,
    /// Width of the indexing ports
    address_bit_width: u64,
    /// Number of parallel ports in the memory
    port_count: u64,
    /// the list of memory banks where the index corresponds to the input port
    banks: Vec<MemoryBank>,
}

#[derive(Debug, Clone)]
pub struct MemoryBank {
    routing: TopLevelRoutingProgram,
    memory_layout: TopLevelMemoryLayout,
}

#[derive(Debug, Clone)]
pub struct TopLevelMemoryLayout {
    mems: Vec<MemoryLayout>,
}

#[derive(Debug, Clone)]
pub enum MemoryLayout {
    Range {
        start: usize,
        finish: usize,
        stride: usize,
    },
}

#[macro_export]
macro_rules! memory {
    ($start:expr ; $end:expr ; $stride:expr) => {
        $crate::structures::MemoryLayout::new(
            $start,
            $end,
            ($stride).into(),
        )
    };

    ($start:expr ; $end:expr) => {
        memory!($start ; $end ; None)
    };
}

#[derive(Debug, Clone)]
pub enum TopLevelRoutingProgram {
    Switch(
        Vec<(Condition, SequenceRoutingProg)>,
        Box<SequenceRoutingProg>,
    ),
    Prog(SequenceRoutingProg),
}

#[derive(Debug, Clone)]
pub enum SequenceRoutingProg {
    Sequence(Vec<TerminalRoutingProgram>),
    Prog(TerminalRoutingProgram),
}

#[derive(Debug, Clone)]
pub enum TerminalRoutingProgram {
    RShift(usize),
    // these all contain the other value
    Add(u64),
    SubPortVal(u64),
    SubValPort(u64),
    Constant(u64),
    Noop,
}

#[derive(Debug, Clone)]
pub enum Condition {
    ComparisonPortVal(u64, ComparisonOperator),
    ComparisonValPort(u64, ComparisonOperator),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

#[derive(Debug, Clone)]
pub enum ComparisonOperator {
    LessThan,
    Equal,
    GreaterThan,
    NotEqual,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Debug, Clone)]
pub enum ShiftDirection {
    Left,
    Right,
}

impl ComparisonOperator {
    pub fn eval(&self, left: &u64, right: &u64) -> bool {
        match self {
            ComparisonOperator::LessThan => left < right,
            ComparisonOperator::Equal => left == right,
            ComparisonOperator::GreaterThan => left > right,
            ComparisonOperator::NotEqual => left != right,
            ComparisonOperator::LessThanOrEqual => left <= right,
            ComparisonOperator::GreaterThanOrEqual => left >= right,
        }
    }
}

impl Condition {
    pub fn eval(&self, port_val: u64) -> bool {
        match self {
            Condition::ComparisonPortVal(val, op) => op.eval(&port_val, val),
            Condition::ComparisonValPort(val, op) => op.eval(val, &port_val),
            Condition::And(c1, c2) => c1.eval(port_val) && c2.eval(port_val),
            Condition::Or(c1, c2) => c1.eval(port_val) || c2.eval(port_val),
            Condition::Not(c1) => !c1.eval(port_val),
        }
    }
}

impl TerminalRoutingProgram {
    pub fn eval(&self, port_val: u64) -> u64 {
        match self {
            TerminalRoutingProgram::Add(v) => (port_val + v),
            TerminalRoutingProgram::SubPortVal(v) => (port_val - v),
            TerminalRoutingProgram::SubValPort(v) => (v - port_val),
            TerminalRoutingProgram::Constant(c) => *c,
            TerminalRoutingProgram::RShift(amount) => port_val >> amount,
            TerminalRoutingProgram::Noop => port_val,
        }
    }
}

impl SequenceRoutingProg {
    pub fn eval(&self, port_val: u64) -> u64 {
        match self {
            SequenceRoutingProg::Sequence(s) => s.iter().fold(port_val, |acc, x| x.eval(acc)),
            SequenceRoutingProg::Prog(p) => p.eval(port_val),
        }
    }
}

impl TopLevelRoutingProgram {
    pub fn eval(&self, port_val: u64) -> u64 {
        match self {
            TopLevelRoutingProgram::Switch(vec, default) => {
                for (cond, prog) in vec.iter() {
                    if cond.eval(port_val) {
                        return prog.eval(port_val);
                    }
                }
                default.eval(port_val)
            }
            TopLevelRoutingProgram::Prog(p) => p.eval(port_val),
        }
    }
}

impl MemoryBank {
    pub fn can_read(&self, index: usize) -> bool {
        let routed_index = self.routing.eval(index as u64);
        let result = self.memory_layout.get(&(routed_index as usize));
        result.map(|x| x == index).unwrap_or(false)
    }
}

impl From<TerminalRoutingProgram> for SequenceRoutingProg {
    fn from(p: TerminalRoutingProgram) -> Self {
        Self::Prog(p)
    }
}

impl From<SequenceRoutingProg> for TopLevelRoutingProgram {
    fn from(p: SequenceRoutingProg) -> Self {
        Self::Prog(p)
    }
}

impl From<TerminalRoutingProgram> for TopLevelRoutingProgram {
    fn from(p: TerminalRoutingProgram) -> Self {
        let p: SequenceRoutingProg = p.into();
        p.into()
    }
}

impl MemoryLayout {
    pub fn new(start: usize, finish: usize, stride: Option<usize>) -> Self {
        let stride = stride.unwrap_or(1);
        assert!(start < finish);
        assert!(stride != 0);

        Self::Range {
            start,
            finish,
            stride,
        }
    }

    #[inline]
    pub fn contains(&self, target: &usize) -> bool {
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => target >= start && target < finish && ((target - start) % stride) == 0,
        }
    }

    pub fn index_of(&self, target: &usize) -> Option<usize> {
        if self.contains(target) {
            let out = match self {
                MemoryLayout::Range { start, stride, .. } => (target - start) / stride,
            };
            return Some(out);
        }
        None
    }

    pub fn size(&self) -> usize {
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => ((finish - start) / stride) + 1,
        }
    }

    pub fn gen_array(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.size());
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => {
                let mut current = *start;
                while current < *finish {
                    out.push(current);
                    current += stride;
                }

                debug_assert!(out
                    .iter()
                    .enumerate()
                    .all(|(i, x)| self.index_of(x).unwrap_or_else(|| panic!("{:?}", x)) == i));
                debug_assert!(out
                    .iter()
                    .enumerate()
                    .all(|(i, x)| self.get(&i).unwrap() == *x));

                out
            }
        }
    }

    pub fn last_idx(&self) -> usize {
        self.size() - 1
    }

    pub fn get(&self, idx: &usize) -> Option<usize> {
        if *idx >= self.size() {
            return None;
        }
        match self {
            MemoryLayout::Range {
                start,
                finish,
                stride,
            } => Some(start + (stride * idx)),
        }
    }
}

impl TopLevelMemoryLayout {
    pub fn contains(&self, target: &usize) -> bool {
        self.mems.iter().any(|x| x.contains(target))
    }

    pub fn index_of(&self, target: &usize) -> Option<usize> {
        let mut idx = 0;

        for mem in self.mems.iter() {
            if mem.contains(target) {
                idx += mem.index_of(target).unwrap();
                return Some(idx);
            } else {
                idx += mem.size();
            }
        }

        None
    }

    pub fn get(&self, idx: &usize) -> Option<usize> {
        let mut bottom_idx = 0_usize;
        for mem in self.mems.iter() {
            if idx - bottom_idx < mem.size() {
                return mem.get(&(idx - bottom_idx));
            } else {
                bottom_idx += mem.size();
            }
        }
        None
    }
}
