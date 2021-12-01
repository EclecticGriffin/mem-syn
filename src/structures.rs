pub struct Component {
    /// The number of slots in the logical memory
    size: u64,
    /// Bitwidth of the stored data elements
    width: u64,
    /// Number of parallel ports in the memory
    port_count: u64,
    /// the list of memory banks where the index corresponds to the input port
    banks: Vec<MemoryBank>,
}

pub struct MemoryBank {
    routing: TopLevelRoutingProgram,
    memory_layout: Vec<u64>,
}

pub enum TopLevelRoutingProgram {
    Switch(Vec<(Condition, RoutingProgram)>, Box<RoutingProgram>),
    Prog(RoutingProgram),
}

pub enum RoutingProgram {
    Sequence(Vec<RoutingProgram>),
    RShift(usize),
    // these all contain the other value
    Add(u64),
    SubPortVal(u64),
    SubValPort(u64),
    Constant(u64),
}

pub enum Condition {
    ComparisonPortVal(u64, ComparisonOperator),
    ComparisonValPort(u64, ComparisonOperator),
    And(Box<Condition>, Box<Condition>),
    Or(Box<Condition>, Box<Condition>),
    Not(Box<Condition>),
}

pub enum ComparisonOperator {
    LessThan,
    Equal,
    GreaterThan,
    NotEqual,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

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

impl RoutingProgram {
    pub fn eval(&self, port_val: u64) -> u64 {
        match self {
            RoutingProgram::Add(v) => (port_val + v),
            RoutingProgram::SubPortVal(v) => (port_val - v),
            RoutingProgram::SubValPort(v) => (v - port_val),
            RoutingProgram::Sequence(s) => s.iter().fold(port_val, |acc, x| x.eval(acc)),
            RoutingProgram::Constant(c) => *c,
            RoutingProgram::RShift(amount) => port_val >> amount,
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
        let index: u64 = index as u64;
        let routed_index = self.routing.eval(index);
        let result = self.memory_layout.get(routed_index as usize);
        result.map(|x| *x == index).unwrap_or(false)
    }
}
