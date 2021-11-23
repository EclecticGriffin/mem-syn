pub struct Component {
    /// The number of slots in the logical memory
    size: u64,
    /// Bitwidth of the stored data elements
    width: u64,
    /// Number of parallel ports in the memory
    port_count: u64,
}

pub struct MemoryBank {
    routing: RoutingProgram,
    memory_layout: Vec<u64>,
}

pub enum RoutingProgram {
    Switch(Vec<(Condition, RoutingProgram)>),
    Conditional {
        port_val: u64,
        comparison_operator: ComparisonOperator,
        then: Box<RoutingProgram>,
        otherwise: Box<RoutingProgram>,
    },
    Shift {
        amount: usize,
        direction: ShiftDirection,
    },
    // these all contain the other value
    Add(u64),
    SubPortVal(u64),
    SubValPort(u64),
    Sequence(Vec<RoutingProgram>),
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
